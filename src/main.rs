#![warn(clippy::nursery)]
use config::{Config, ServiceConfig};
use log::*;
use providers::DnsProvider;
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr},
};

mod config;
mod providers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/etc/dynsix/config.toml".to_string());
    let mut config = Config::load(config_path)?;

    // Initialize all configured providers, skipping if initialization fails
    //
    // Providers are not initialized on demand because some may require a form of
    // rate limiting
    let mut providers: HashMap<String, Box<dyn DnsProvider>> = HashMap::new();
    debug!("Initializing Providers");
    for (name, config) in config.provider.iter() {
        match providers::get_provider(name, config) {
            Ok(provider) => {
                providers.insert(name.to_string(), Box::new(provider));
                info!("Provider {name} initialized");
            }
            Err(e) => {
                error!("Failed to initialize provider {name}: {e}")
            }
        }
    }

    // Determine default provider if none is explicetly set and only one is provided
    if config.default_provider.is_none() {
        if providers.len() == 1 {
            config.default_provider = providers.keys().next().cloned();
            info!(
                "Provider {} determined as the default provider.",
                config.default_provider.as_ref().unwrap()
            );
        } else {
            warn!("Could not determine default provider. This is only a problem if a service defines no provider.");
        }
    }

    // Fetching global IP
    let prefix = get_global_ip(&config.query_server).await?;

    // Execute provider on each service
    for service in config.service.iter() {
        debug!("Working on service {}.{}", service.1.name, service.1.fqdn);
        match process_service(service, &mut providers, &config, prefix).await {
            Ok(_) => {}
            Err(e) => error!("Error occured while running service: {e}"),
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct EchoIpAnswer {
    ip: Ipv6Addr,
}

async fn get_global_ip(ip_query_server: &str) -> Result<Ipv6Addr, reqwest::Error> {
    let ipv6_client = Client::builder()
        .local_address("::0".parse::<IpAddr>().unwrap())
        .build()?;

    let response = ipv6_client
        .get(ip_query_server)
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<EchoIpAnswer>()
        .await?;

    Ok(response.ip)
}

#[derive(thiserror::Error, Debug)]
enum ServiceError {
    #[error(
        "no provider was defined for {service_name} and no default provider could be determined"
    )]
    NoProviderDefined { service_name: String },

    #[error(
        "provider {provider_name} defined for {service_name} is unknown or failed to initialize"
    )]
    ProviderUnknown {
        service_name: String,
        provider_name: String,
    },

    #[error("{0}")]
    ProviderError(#[from] providers::ProviderError),

    #[error("Unknown error")]
    Unknown,
}

async fn process_service(
    (name, service): (&String, &ServiceConfig),
    providers: &mut HashMap<String, Box<dyn DnsProvider>>,
    config: &Config,
    prefix: Ipv6Addr,
) -> Result<(), ServiceError> {
    let provider_name = if let Some(name) = service
        .provider
        .as_ref()
        .or(config.default_provider.as_ref())
    {
        name
    } else {
        return Err(ServiceError::NoProviderDefined {
            service_name: name.into(),
        });
    };
    if let Some(provider) = providers.get_mut(provider_name) {
        let service_ip = merge_ips(prefix, service.suffix);
        if let Some(existing_record) = provider
            .get_aaaa_record(&service.fqdn, &service.name)
            .await?
        {
            debug!(
                "Record {}.{} exists and is set to {existing_record:?}",
                service.name, service.fqdn
            );

            if service_ip != existing_record {
                provider
                    .update_aaaa_record(&service.fqdn, &service.name, service_ip)
                    .await?;
            }
        } else {
            debug!("No record for {}.{} exists", service.name, service.fqdn);
            provider
                .set_aaaa_record(&service.fqdn, &service.name, service_ip)
                .await?;
        }
    } else {
        return Err(ServiceError::ProviderUnknown {
            service_name: name.into(),
            provider_name: provider_name.into(),
        });
    };

    Ok(())
}

const fn merge_ips(prefix: Ipv6Addr, suffix: Ipv6Addr) -> Ipv6Addr {
    let prefix_segments = prefix.segments();
    let suffix_segments = suffix.segments();

    Ipv6Addr::new(
        prefix_segments[0],
        prefix_segments[1],
        prefix_segments[2],
        prefix_segments[3],
        suffix_segments[4],
        suffix_segments[5],
        suffix_segments[6],
        suffix_segments[7],
    )
}
