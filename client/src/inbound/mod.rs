use std::{net::IpAddr, sync::Arc};

use anyhow::Result;
use reqwest::Client;
use tracing::info;

use crate::inbound::{client::create_client, dns::configure_yandex_dns};

mod client;
mod dns;
pub mod interface;
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0";

/// Создаём исходный клиент для запросов к провайдерам внутри белых списков
pub async fn create_inbound_client(ip_interface: IpAddr) -> Result<Client>
{
  info!("Creating inbound client...");
  let dns = configure_yandex_dns()?;
  info!("Yandex DNS configured successfully");

  let client = create_client(ip_interface, Arc::new(dns))?;
  info!(
    "Inbound client created successfully with IP interface: {}",
    ip_interface
  );

  Ok(client)
}
