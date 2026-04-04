use std::{net::IpAddr, sync::Arc};

use anyhow::Result;
use reqwest::{Client, dns::Resolve};

use crate::inbound::USER_AGENT;

/// Создаётся клиент с определённым DNS-резолвером и IP интерфейсом
pub fn create_client(
  addr_interface: IpAddr,
  resolver: Arc<dyn Resolve>,
) -> Result<Client>
{
  let builder = reqwest::ClientBuilder::new()
    .no_proxy()
    .hickory_dns(true)
    .user_agent(USER_AGENT)
    .local_address(addr_interface)
    .dns_resolver(resolver);

  let client = builder.build()?;

  Ok(client)
}
