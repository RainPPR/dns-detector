mod models;
mod dns_client;

use clap::Parser;
use models::{DnsServersConfig, SiteServersConfig, DnsMethod};
use dns_client::{DnsResolver, measure_doh_latency};
use indicatif::{ProgressBar, ProgressStyle};
use anyhow::{Result, Context};
use futures::future::join_all;

use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "dns_servers.json")]
    dns_file: String,

    #[arg(short, long, default_value = "site_servers.json")]
    sites_file: String,

    #[arg(short, long, default_value = "results.csv")]
    output: String,

    #[arg(short, long, default_value = "16")]
    concurrency: usize,
}

struct TestResult {
    dns_name: String,
    method: String,
    address: String,
    results: Vec<(String, Option<String>, i64)>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let dns_data = std::fs::read_to_string(&args.dns_file)
        .with_context(|| format!("Failed to read {}", args.dns_file))?;
    let dns_config: DnsServersConfig = serde_json::from_str(&dns_data)?;

    let site_data = std::fs::read_to_string(&args.sites_file)
        .with_context(|| format!("Failed to read {}", args.sites_file))?;
    let site_config: SiteServersConfig = serde_json::from_str(&site_data)?;

    let mut method_configs = Vec::new();
    for server in &dns_config.servers {
        for ip in &server.ipv4 {
            method_configs.push((server.clone(), DnsMethod::IPv4(ip.clone())));
        }
        for ip in &server.ipv6 {
            method_configs.push((server.clone(), DnsMethod::IPv6(ip.clone())));
        }
        for url in &server.doh {
            method_configs.push((server.clone(), DnsMethod::DoH(url.clone())));
        }
    }

    let all_sites: Vec<String> = site_config.servers.iter()
        .flat_map(|s| s.url.clone())
        .collect();

    let total_tasks = method_configs.len() * all_sites.len();
    let pb = ProgressBar::new(total_tasks as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
        .unwrap());

    let semaphore = Arc::new(Semaphore::new(args.concurrency));
    let pb = Arc::new(pb);
    let mut tasks = Vec::new();

    for (server, method) in method_configs {
        let all_sites = all_sites.clone();
        let semaphore = Arc::clone(&semaphore);
        let pb = Arc::clone(&pb);

        tasks.push(tokio::spawn(async move {
            let mut site_results = Vec::new();
            let resolver = if !matches!(method, DnsMethod::DoH(_)) {
                DnsResolver::new(&method).await.ok()
            } else {
                None
            };

            for site in all_sites {
                let permit = semaphore.acquire().await.unwrap();
                pb.set_message(format!("{} -> {}", server.name, site));

                let (ip, latency) = match &method {
                    DnsMethod::DoH(url) => measure_doh_latency(url, &site).await,
                    _ => {
                        if let Some(res) = &resolver {
                            res.measure_latency(&site).await
                        } else {
                            (None, -1)
                        }
                    }
                };

                site_results.push((site, ip, latency));
                pb.inc(1);
                drop(permit);
            }

            TestResult {
                dns_name: server.name.clone(),
                method: method.name().to_string(),
                address: method.address().to_string(),
                results: site_results,
            }
        }));
    }

    let final_results: Vec<TestResult> = join_all(tasks)
        .await
        .into_iter()
        .map(|res| res.unwrap())
        .collect();

    pb.finish_with_message("Testing complete");

    // Console output summary
    println!("\n{:<30} {:<10} {:<15} {:<10} {:<10} {:<10}", "DNS Name", "Method", "Avg Latency", "Max", "Min", "Failures");
    println!("{}", "-".repeat(95));

    for res in &final_results {
        let valid_latencies: Vec<i64> = res.results.iter()
            .map(|(_, _, lat)| *lat)
            .filter(|&l| l != -1)
            .collect();

        let count = valid_latencies.len();
        let total: i64 = valid_latencies.iter().sum();
        let failures = res.results.len() - count;

        if count > 0 {
            let avg = total as f64 / count as f64;
            let max = *valid_latencies.iter().max().unwrap();
            let min = *valid_latencies.iter().min().unwrap();
            println!("{:<30} {:<10} {:<15.2} {:<10} {:<10} {:<10}", 
                res.dns_name, res.method, avg, max, min, failures);
        } else {
            println!("{:<30} {:<10} {:<15} {:<10} {:<10} {:<10}", 
                res.dns_name, res.method, "N/A", "N/A", "N/A", failures);
        }
    }

    // CSV Output
    let mut wtr = csv::Writer::from_path(&args.output)?;
    
    // Header
    let mut header = vec!["DNS Name".to_string(), "Method".to_string(), "Address/URL".to_string()];
    for site in &all_sites {
        header.push(format!("{}_IP", site));
        header.push(format!("{}_Latency", site));
    }
    wtr.write_record(&header)?;

    // Data
    for res in &final_results {
        let mut row = vec![res.dns_name.clone(), res.method.clone(), res.address.clone()];
        // Note: Row data must follow all_sites order
        for site in &all_sites {
            if let Some((_, ip, lat)) = res.results.iter().find(|(s, _, _)| s == site) {
                row.push(ip.clone().unwrap_or_else(|| "-".to_string()));
                row.push(lat.to_string());
            } else {
                row.push("-".to_string());
                row.push("-1".to_string());
            }
        }
        wtr.write_record(&row)?;
    }
    wtr.flush()?;

    println!("\nDetailed results saved to {}", args.output);

    Ok(())
}

