use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use webrtc_util::Conn;

pub fn proxy_flow(
  flow_name: String,
  from_addr: SocketAddr,
  to_addr: SocketAddr,
  cancellation_token: CancellationToken,
  from_flow: Arc<dyn Conn + Send + Sync>,
  to_flow: Arc<dyn Conn + Send + Sync>
) -> JoinHandle<anyhow::Result<()>> {
  tokio::spawn(async move {
    let mut buf = [0u8; 2048];

    loop {
      match from_flow.recv(&mut buf).await {
        Ok(n) if n > 0 => {
          debug!("[{}] Received {} bytes from {}", flow_name, n, from_addr);
          if n >= buf.len() {
            warn!("[{}] Packet from {} is too large for buffer ({})", flow_name, from_addr, n);
          }
          if let Err(e) = to_flow.send(&buf[..n]).await {
            warn!("[{}] Error sending to {} from {}: {}", flow_name, to_addr, from_addr, e);
            break;
          }
          debug!("[{}] Send {} bytes into {}", flow_name, n, to_addr);
        }
        _ => break,
      }
    }

    cancellation_token.cancel();
    Ok(())
  })
}