use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use webrtc_util::Conn;

/// Более высокоуровневая структура для прокси-взаимодействия
pub struct ProxyBridge
{
  pub flow_name: String,
  pub cancellation_token: CancellationToken,
  pub local_conn: Arc<dyn Conn + Send + Sync>,
  pub remote_conn: Arc<dyn Conn + Send + Sync>,
  pub client_addr_cache: Option<Arc<RwLock<Option<SocketAddr>>>>,
}

impl ProxyBridge
{
  pub fn new(
    flow_name: String,
    cancellation_token: CancellationToken,
    local_conn: Arc<dyn Conn + Send + Sync>,
    remote_conn: Arc<dyn Conn + Send + Sync>,
    client_addr_cache: Option<Arc<RwLock<Option<SocketAddr>>>>,
  ) -> Self
  {
    Self {
      flow_name,
      cancellation_token,
      local_conn,
      remote_conn,
      client_addr_cache,
    }
  }

  pub fn spawn(&self) -> Result<(JoinHandle<Result<()>>, JoinHandle<Result<()>>)>
  {
    let upstream = self.run_upstream()?;
    let downstream = self.run_downstream()?;

    Ok((upstream, downstream))
  }

  fn run_upstream(&self) -> Result<JoinHandle<Result<()>>>
  {
    let flow_name = format!("{}-UP", &self.flow_name);
    let cancellation_token = self.cancellation_token.clone();
    let local_conn = self.local_conn.clone();
    let remote_conn = self.remote_conn.clone();
    let local_addr = local_conn.remote_addr().unwrap_or(local_conn.local_addr()?);
    let remote_addr = remote_conn
      .remote_addr()
      .unwrap_or(remote_conn.local_addr()?);
    let client_addr_cache = self.client_addr_cache.clone();

    Ok(proxy_flow(
      flow_name,
      cancellation_token,
      local_addr,
      remote_addr,
      local_conn,
      remote_conn,
      client_addr_cache,
      None,
    ))
  }

  fn run_downstream(&self) -> Result<JoinHandle<Result<()>>>
  {
    let flow_name = format!("{}-DOWN", &self.flow_name);
    let cancellation_token = self.cancellation_token.clone();
    let local_conn = self.local_conn.clone();
    let remote_conn = self.remote_conn.clone();
    let local_addr = local_conn.remote_addr().unwrap_or(local_conn.local_addr()?);
    let remote_addr = remote_conn
      .remote_addr()
      .unwrap_or(remote_conn.local_addr()?);
    let client_addr_cache = self.client_addr_cache.clone();

    Ok(proxy_flow(
      flow_name,
      cancellation_token,
      remote_addr,
      local_addr,
      remote_conn,
      local_conn,
      None,
      client_addr_cache,
    ))
  }
}

/// Низкоуровневая абстракция
pub fn proxy_flow(
  flow_name: String,
  cancellation_token: CancellationToken,

  _from_addr: SocketAddr,
  to_addr: SocketAddr,

  from_flow: Arc<dyn Conn + Send + Sync>,
  to_flow: Arc<dyn Conn + Send + Sync>,

  from_cache: Option<Arc<RwLock<Option<SocketAddr>>>>,
  to_cache: Option<Arc<RwLock<Option<SocketAddr>>>>,
) -> JoinHandle<Result<()>>
{
  tokio::spawn(async move {
    let mut buf = [0u8; 2048];

    loop {
      match from_flow.recv_from(&mut buf).await {
        Ok((n, src)) if n > 0 => {
          if let Some(cache) = &from_cache {
            cache.write().await.replace(src);
          }

          debug!("[{}] Received {} bytes from {}", flow_name, n, src);
          if n >= buf.len() {
            warn!(
              "[{}] Packet from {} is too large for buffer ({})",
              flow_name, src, n
            );
          }
          if let Some(cache) = &to_cache {
            let dest = cache.read().await.unwrap_or(to_addr);
            if let Err(e) = to_flow.send_to(&buf[..n], dest).await {
              warn!(
                "[{}] Error sending to {} from {}: {}",
                flow_name, dest, src, e
              );
              break;
            }
            debug!("[{}] Send {} bytes into {}", flow_name, n, dest);
          } else {
            if let Err(e) = to_flow.send(&buf[..n]).await {
              warn!(
                "[{}] Error sending to {} from {}: {}",
                flow_name, to_addr, src, e
              );
              break;
            }
            debug!("[{}] Send {} bytes into {}", flow_name, n, to_addr);
          }
        }
        _ => break,
      }
    }

    cancellation_token.cancel();
    Ok(())
  })
}
