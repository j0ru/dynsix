use std::{collections::HashMap, error::Error, net::Ipv6Addr, ops::Deref, path::Path};

use serde::Deserialize;
use toml::Value;

#[derive(Debug, Deserialize)]
pub struct ProviderConfig(Value);

impl Deref for ProviderConfig {
    fn deref(&self) -> &Self::Target {
        &self.0
    }

    type Target = Value;
}

#[derive(Deserialize, Debug)]
pub struct Config {
    /// the Url of an [EchoIP](https://github.com/mpolden/echoip) compatible service
    #[serde(default = "default_query_server")]
    pub query_server: String,

    pub default_provider: Option<String>,

    pub service: HashMap<String, ServiceConfig>,
    pub provider: HashMap<String, ProviderConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServiceConfig {
    pub suffix: Ipv6Addr,
    pub name: String,
    pub fqdn: String,
    pub ttl: u32,
    pub provider: Option<String>,
}

impl Config {
    pub fn load<P>(path: P) -> Result<Self, Box<dyn Error>>
    where
        P: AsRef<Path>,
    {
        let config_raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&config_raw)?)
    }
}

// Default implementations
fn default_query_server() -> String {
    "https://ifconfig.co".to_string()
}
