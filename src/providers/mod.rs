use crate::config::ProviderConfig;
use async_trait::async_trait;
use std::{error::Error, net::Ipv6Addr};

mod gandi;

#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("Authentication with the provider failed")]
    Unauthenticated,

    #[error("Authentication with the provider failed")]
    Unknown,
}

#[async_trait]
pub trait DnsProvider: Send + core::fmt::Debug {
    fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;
    async fn get_aaaa_record(
        &mut self,
        fqdn: &str,
        name: &str,
    ) -> Result<Option<Ipv6Addr>, ProviderError>;
    async fn set_aaaa_record(
        &mut self,
        fqdn: &str,
        name: &str,
        ip: Ipv6Addr,
    ) -> Result<(), ProviderError>;
    async fn update_aaaa_record(
        &mut self,
        fqdn: &str,
        name: &str,
        ip: Ipv6Addr,
    ) -> Result<(), ProviderError>;
}

pub fn get_provider(
    name: &str,
    config: &ProviderConfig,
) -> Result<impl DnsProvider, Box<dyn Error>> {
    match name {
        "gandi" => gandi::Gandi::new(config),
        _ => panic!("unknown provider in config"),
    }
}
