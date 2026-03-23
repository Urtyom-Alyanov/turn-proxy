use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use webrtc_util::Conn;

use crate::proxy_process::proxy_flow::{ProxyBridge, proxy_flow};

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

  let idle_timeout = Duration::from_hours(6);

  let token = CancellationToken::new();

  let proxy_bridge = ProxyBridge::new(
    String::new(),
    token.clone(),
    dtls_conn.clone(),
    socket_arc,
    None,
  );

  let (upstream, downstream) = proxy_bridge.spawn()?;

  let result = tokio::time::timeout(idle_timeout, async {
    tokio::select! {
      _ = upstream => { debug!("Client task finished"); },
      _ = downstream => { debug!("Target task finished"); },
    }
  })
  .await;

  if result.is_err() {
    info!("Connection timed out due to inactivity");
    token.cancel();
  }

  let _ = dtls_conn.close().await;
  Ok(())
}
