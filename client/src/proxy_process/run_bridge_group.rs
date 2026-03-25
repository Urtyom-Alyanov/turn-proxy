use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::{net::UdpSocket};
use tokio_util::sync::CancellationToken;
use webrtc_util::Conn;
use turn_proxy_lib::proxy::run_proxy_bridge;

pub async fn run_bridge_thread(
  thread_num: usize,
  listen_conn: Arc<UdpSocket>,
  remote_conn: Arc<dyn Conn + Send + Sync>,
  token: CancellationToken,
) -> Result<()>
{
  let thread_name = format!("T{}", thread_num);
  
  run_proxy_bridge(
    thread_name,
    token,
    Some(Duration::from_secs(150)),
    listen_conn,
    remote_conn,
    true
  ).await
}
