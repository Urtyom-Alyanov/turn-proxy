use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;

use crate::inbound::{
  client::create_client, dns::configure_yandex_dns, interface::get_current_interface,
};

mod client;
mod dns;
mod interface;
pub const USER_AGENT: &str =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:144.0) Gecko/20100101 Firefox/144.0";

/// Создаём исходный клиент для запросов к провайдерам внутри белых списков
pub async fn create_inbound_client() -> Result<Client>
{
  let ip_interface = get_current_interface().await?;
  let dns = configure_yandex_dns()?;

  let client = create_client(ip_interface, Arc::new(dns))?;

  Ok(client)
}
