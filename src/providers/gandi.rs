use std::ops::Deref;

use super::{DnsProvider, ProviderError};
use async_trait::async_trait;
use log::*;
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use tokio::time::{sleep_until, Duration, Instant};

const REQUEST_INTERVAL: Duration = Duration::new(60, 0);

#[derive(Debug)]
pub struct Gandi {
    client: Client,
    last_request: Option<Instant>,
}

#[derive(Debug, Deserialize)]
struct Config {
    pub token: String,
}

#[derive(Deserialize)]
struct GandiGetResponse {
    rrset_values: Vec<String>,
}

#[async_trait]
impl DnsProvider for Gandi {
    fn new(config: &crate::config::ProviderConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Config = config.deref().clone().try_into()?;

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("ApiKey {}", config.token).parse().unwrap(),
        );
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());

        let client = Client::builder().default_headers(headers).build()?;

        Ok(Self {
            client,
            last_request: None,
        })
    }

    async fn get_aaaa_record(
        &mut self,
        fqdn: &str,
        name: &str,
    ) -> Result<Option<std::net::Ipv6Addr>, ProviderError> {
        self.wait_for_rate_limit().await;
        match self
            .client
            .get(format!(
                "https://api.gandi.net/v5/livedns/domains/{fqdn}/records/{name}/AAAA"
            ))
            .send()
            .await
        {
            Ok(resp) => {
                debug!("{:?}", resp);
                if let Ok(content) = resp.json::<GandiGetResponse>().await {
                    return Ok(content.rrset_values.get(0).map(|x| x.parse().unwrap()));
                } else {
                    return Ok(None);
                }
            }
            Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => return Ok(None),
            Err(e) if e.status() == Some(StatusCode::FORBIDDEN) => {
                return Err(ProviderError::Unauthenticated)
            }
            Err(e) => {
                error!("{e}");
                return Err(ProviderError::Unknown);
            }
        }
    }

    async fn set_aaaa_record(
        &mut self,
        fqdn: &str,
        name: &str,
        ip: std::net::Ipv6Addr,
    ) -> Result<(), ProviderError> {
        self.wait_for_rate_limit().await;
        match self
            .client
            .post(format!(
                "https://api.gandi.net/v5/livedns/domains/{fqdn}/records/{name}/AAAA"
            ))
            .json(&json!({
                "rrset_values": vec![ip.to_string()],
                "rrset_ttl": 600,
            }))
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(ProviderError::Unknown),
        }
    }

    async fn update_aaaa_record(
        &mut self,
        fqdn: &str,
        name: &str,
        ip: std::net::Ipv6Addr,
    ) -> Result<(), ProviderError> {
        self.wait_for_rate_limit().await;
        match self
            .client
            .put(format!(
                "https://api.gandi.net/v5/livedns/domains/{fqdn}/records/{name}/AAAA"
            ))
            .json(&json!({
                "rrset_values": vec![ip.to_string()],
                "rrset_ttl": 600,
            }))
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(ProviderError::Unknown),
        }
    }
}

impl Gandi {
    /// Ensures that at least REQUEST_INTERVAL is between last call and returning
    async fn wait_for_rate_limit(&mut self) {
        if let Some(last_request) = self.last_request {
            if let Some(target_instant) = last_request.checked_add(REQUEST_INTERVAL) {
                sleep_until(target_instant).await
            } else {
                warn!("Failed to set timer for Gandi provider");
            }
        }
        self.last_request = Some(Instant::now());
    }
}
