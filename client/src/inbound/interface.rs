use std::net::IpAddr;

use anyhow::{Result, anyhow};
use net_route::Handle;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};

/// Получаем стандартный на момент подключения к провайдеру IP интерфейса, чтобы
/// при включении туннелирования не было такого, что запросы идут не туда куда
/// надо
pub async fn get_current_interface() -> Result<IpAddr>
{
  let handle = Handle::new()?;
  let routes = handle.list().await?;

  let default_route = routes
    .iter()
    .find(|r| r.destination == "0.0.0.0".parse::<IpAddr>().unwrap() && r.prefix == 0)
    .expect("Default gateway is not founded (network has working?)");

  let iface_index = default_route
    .ifindex
    .ok_or_else(|| anyhow!("Default gatefay index has not seted"))?;

  let interfaces = NetworkInterface::show()?;
  let active_iface = interfaces
    .into_iter()
    .find(|iface| iface.index == iface_index)
    .expect("Interface of gateway has not found in devices");

  let ip = active_iface
    .addr
    .iter()
    .find(|a| a.ip().is_ipv4())
    .map(|a| a.ip());

  Ok(ip.ok_or_else(|| anyhow!("Unexpected error on ip interface getting"))?)
}
