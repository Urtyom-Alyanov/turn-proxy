use crate::proxy_process::proxy_flow::proxy_flow;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use anyhow::{Result,Context};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
use webrtc_util::Conn;

pub async fn handle_encrypted_udp_connection(dtls_conn: Arc<dyn Conn + Send + Sync>, proxy_addr: SocketAddr) -> Result<()> {
  let target_socket = UdpSocket::bind("0.0.0.0:0").await
    .context("Failed to bind local UDP socket")?;

  debug!("Local socket {} successfully bound", target_socket.local_addr()?);

  if let Err(e) = target_socket.connect(proxy_addr).await {
    error!("Failed to connect to target addr {}: {:?}", proxy_addr, e);
    return Err(e).context("Failed to connect to target addr");
  }

  debug!("Successfully connected to target {} from {}", target_socket.peer_addr()?, target_socket.local_addr()?);

  let socket_arc = Arc::new(target_socket);

  let from_dtls = Arc::clone(&dtls_conn);
  let to_dtls = Arc::clone(&dtls_conn);
  let from_socket = Arc::clone(&socket_arc);
  let to_socket = Arc::clone(&socket_arc);

  let idle_timeout = Duration::from_hours(6);

  let token = CancellationToken::new();
  let t1 = token.clone();
  let t2 = token.clone();

  let client_to_proxy = proxy_flow(
    "UPSTREAM".to_owned(),
    t1,
    to_socket.peer_addr()?,
    to_socket.local_addr()?,
    from_dtls,
    to_socket
  );
  let target_to_proxy = proxy_flow(
    "DOWNSTREAM".to_owned(),
    t2,
    from_socket.local_addr()?,
    from_socket.peer_addr()?,
    from_socket,
    to_dtls
  );

  let result = tokio::time::timeout(idle_timeout, async {
    tokio::select! {
      _ = client_to_proxy => { debug!("Client task finished"); },
      _ = target_to_proxy => { debug!("Target task finished"); },
    }
  }).await;

  if result.is_err() {
    info!("Connection timed out due to inactivity");
    token.cancel();
  }

  let _ = dtls_conn.close().await;
  Ok(())
}