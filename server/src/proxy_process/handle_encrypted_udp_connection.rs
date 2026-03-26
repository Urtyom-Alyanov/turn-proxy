use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};
use webrtc_util::Conn;
use turn_proxy_lib::proxy::run_proxy_bridge;

pub async fn handle_encrypted_udp_connection(
  dtls_conn: Arc<dyn Conn + Send + Sync>,
  proxy_addr: SocketAddr,
) -> Result<()>
{
  let target_socket = UdpSocket::bind("0.0.0.0:0")
    .await
    .context("Failed to bind local UDP socket")?;

  debug!(
    "Local socket {} successfully bound",
    target_socket.local_addr()?
  );

  if let Err(e) = target_socket.connect(proxy_addr).await {
    error!("Failed to connect to target addr {}: {:?}", proxy_addr, e);
    return Err(e).context("Failed to connect to target addr");
  }

  debug!(
    "Successfully connected to target {} from {}",
    target_socket.peer_addr()?,
    target_socket.local_addr()?
  );

  let socket_arc = Arc::new(target_socket);

  // Через 2 минуты без данных TURN (по крайней мере от ВК) обрывает соединение,
  // поэтому оставлять его активным бессмысленно и даже вредит, так как не
  // остаётся свободных портов. Так же сам WireGuard отправляет запросы с
  // рукопожатием каждые 2 минуты
  let idle_timeout = Duration::from_secs(150);

  let token = CancellationToken::new();

  run_proxy_bridge(
    "SERVER".to_owned(),
    token,
    Some(idle_timeout),
    dtls_conn,
    socket_arc,
    false
  ).await
}
