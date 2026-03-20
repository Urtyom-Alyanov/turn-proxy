use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use webrtc_util::Conn;

pub struct ProxyBridge {
  pub flow_name: String,
  pub cancellation_token: CancellationToken,
}

impl ProxyBridge {
  pub fn new(flow_name: String, cancellation_token: CancellationToken) -> Self {
    Self {
      flow_name,
      cancellation_token,
    }
  }

  pub async fn run_upstream(
    &self,
    local_conn: Arc<dyn Conn + Send + Sync>,
    remote_conn: Arc<dyn Conn + Send + Sync>,
  ) -> Result<JoinHandle<Result<()>>> {
    let flow_name = format!("{}-UP", &self.flow_name);
    let cancellation_token = self.cancellation_token.clone();
    let local_addr = local_conn.local_addr()?;
    let remote_addr = remote_conn.local_addr()?;

    Ok(proxy_flow(
      flow_name,
      cancellation_token,

      local_addr,
      remote_addr,

      local_conn,
      remote_conn,
    ))
  }

  pub async fn run_downstream(
    &self,
    local_conn: Arc<dyn Conn + Send + Sync>,
    remote_conn: Arc<dyn Conn + Send + Sync>,
  ) -> Result<JoinHandle<Result<()>>> {
    let flow_name = format!("{}-DOWN", &self.flow_name);
    let cancellation_token = self.cancellation_token.clone();
    let local_addr = local_conn.local_addr()?;
    let remote_addr = remote_conn.local_addr()?;

    Ok(proxy_flow(
      flow_name,
      cancellation_token,

      remote_addr,
      local_addr,

      remote_conn,
      local_conn,
    ))
  }
}

/// Низкоуровневая абстракция
pub fn proxy_flow(
  flow_name: String,
  cancellation_token: CancellationToken,

  from_addr: SocketAddr,
  to_addr: SocketAddr,

  from_flow: Arc<dyn Conn + Send + Sync>,
  to_flow: Arc<dyn Conn + Send + Sync>
) -> JoinHandle<Result<()>> {
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