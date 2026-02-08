use anyhow::Result;
use hickory_resolver::config::{ResolverConfig, ResolverOpts, NameServerConfig, Protocol};
use hickory_resolver::TokioAsyncResolver;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use url;
use crate::models::DnsMethod;

pub struct DnsResolver {
    inner: TokioAsyncResolver,
}

const TIMEOUT: Duration = Duration::from_secs(3);
const RETRY_THRESHOLD: i64 = 500;
const MAX_RETRIES: u32 = 3;

impl DnsResolver {
    pub async fn new(method: &DnsMethod) -> Result<Self> {
        let mut config = ResolverConfig::new();
        let mut opts = ResolverOpts::default();
        opts.timeout = TIMEOUT;
        opts.attempts = 1;

        match method {
            DnsMethod::IPv4(addr) => {
                let ip: IpAddr = addr.parse()?;
                config.add_name_server(NameServerConfig::new(SocketAddr::new(ip, 53), Protocol::Udp));
            }
            DnsMethod::IPv6(addr) => {
                let ip: IpAddr = addr.parse()?;
                config.add_name_server(NameServerConfig::new(SocketAddr::new(ip, 53), Protocol::Udp));
            }
            DnsMethod::DoH(_) => {}
        }

        let resolver = TokioAsyncResolver::tokio(config, opts);
        Ok(Self { inner: resolver })
    }

    async fn resolve_once(&self, domain: &str) -> (Option<String>, i64) {
        let start = Instant::now();
        let domain_with_dot = if domain.ends_with('.') {
            domain.to_string()
        } else {
            format!("{}.", domain)
        };

        match timeout(TIMEOUT, self.inner.lookup_ip(domain_with_dot)).await {
            Ok(Ok(response)) => {
                let latency = start.elapsed().as_millis() as i64;
                let ip = response.iter().next().map(|ip| ip.to_string());
                (ip, latency)
            }
            _ => (None, -1),
        }
    }

    pub async fn measure_latency(&self, domain: &str) -> (Option<String>, i64) {
        let mut best_lat = -1;
        let mut best_ip = None;

        for _ in 0..MAX_RETRIES {
            let (ip, lat) = self.resolve_once(domain).await;
            if lat != -1 {
                if best_lat == -1 || lat < best_lat {
                    best_lat = lat;
                    best_ip = ip;
                }
                if lat <= RETRY_THRESHOLD {
                    break;
                }
            }
        }
        (best_ip, best_lat)
    }
}

pub async fn measure_doh_latency(url_str: &str, domain: &str) -> (Option<String>, i64) {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .unwrap();

    let mut best_lat = -1;
    let mut best_ip = None;

    for _ in 0..MAX_RETRIES {
        let start = Instant::now();
        let url = match url::Url::parse_with_params(url_str, &[("name", domain), ("type", "A")]) {
            Ok(u) => u,
            Err(_) => break,
        };

        let res = client.get(url)
            .header("Accept", "application/dns-json")
            .send()
            .await;

        if let Ok(response) = res {
            if response.status().is_success() {
                let latency = start.elapsed().as_millis() as i64;
                let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
                let ip = body["Answer"]
                    .as_array()
                    .and_then(|arr| arr.iter().find(|v| v["type"] == 1))
                    .and_then(|v| v["data"].as_str())
                    .map(|s| s.to_string());

                let ip_val = ip.or(Some("N/A".to_string()));
                if best_lat == -1 || latency < best_lat {
                    best_lat = latency;
                    best_ip = ip_val;
                }
                if latency <= RETRY_THRESHOLD {
                    break;
                }
            }
        }
    }
    (best_ip, best_lat)
}
