use std::{net::IpAddr, sync::Arc};

use anyhow::Result;
use reqwest::{Client, dns::Resolve};

/// Создаётся клиент с определённым DNS-резолвером и IP интерфейсом
pub fn create_client(
  addr_interface: IpAddr,
  resolver: Arc<dyn Resolve>,
  client_user_agent: &str,
) -> Result<Client>
{
  let builder = reqwest::ClientBuilder::new()
    .no_proxy()
    .cookie_store(true)
    .hickory_dns(true)
    .user_agent(client_user_agent)
    .local_address(addr_interface)
    .dns_resolver(resolver);

  let client = builder.build()?;

  Ok(client)
}
