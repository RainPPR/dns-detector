use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DnsServer {
    pub name: String,
    pub location: String,
    pub description: String,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    #[serde(default)]
    pub doh: Vec<String>,
    #[serde(default)]
    pub dot: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DnsServersConfig {
    pub servers: Vec<DnsServer>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SiteServer {
    pub name: String,
    pub description: String,
    pub url: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SiteServersConfig {
    pub servers: Vec<SiteServer>,
}

#[derive(Debug, Clone)]
pub enum DnsMethod {
    IPv4(String),
    IPv6(String),
    DoH(String),
}

impl DnsMethod {
    pub fn name(&self) -> &str {
        match self {
            DnsMethod::IPv4(_) => "IPv4",
            DnsMethod::IPv6(_) => "IPv6",
            DnsMethod::DoH(_) => "DoH",
        }
    }

    pub fn address(&self) -> &str {
        match self {
            DnsMethod::IPv4(addr) => addr,
            DnsMethod::IPv6(addr) => addr,
            DnsMethod::DoH(url) => url,
        }
    }
}
