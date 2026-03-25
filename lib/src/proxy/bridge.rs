use std::{net::SocketAddr, sync::Arc};
use std::time::Duration;
use anyhow::Result;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use webrtc_util::Conn;
use futures_util::future::select_all;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use crate::proxy::flow::proxy_flow;

pub struct ProxyBridge
{
  pub flow_name: String,
  pub cancellation_token: CancellationToken,
  pub local_conn: Arc<dyn Conn + Send + Sync>,
  pub remote_conn: Arc<dyn Conn + Send + Sync>,
  pub client_addr_cache: Option<Arc<RwLock<Option<SocketAddr>>>>,
  pub idle_timeout: Option<Duration>,
}

impl ProxyBridge
{
  pub fn new(
    flow_name: String,
    cancellation_token: CancellationToken,
    local_conn: Arc<dyn Conn + Send + Sync>,
    remote_conn: Arc<dyn Conn + Send + Sync>,
    client_addr_cache: Option<Arc<RwLock<Option<SocketAddr>>>>,
    idle_timeout: Option<Duration>
  ) -> Self
  {
    Self {
      flow_name,
      cancellation_token,
      local_conn,
      remote_conn,
      client_addr_cache,
      idle_timeout
    }
  }

  fn spawn(&self) -> Result<(JoinHandle<Result<()>>, JoinHandle<Result<()>>)>
  {
    let upstream = self.run_upstream()?;
    let downstream = self.run_downstream()?;

    Ok((upstream, downstream))
  }

  pub async fn run(&self) -> Result<()>
  {
    let (up, down) = self.spawn()?;

    tokio::select! {
      res = select_all(vec![up, down]) => {
        let (result, index, _) = res;
        let direction = if index == 0 { "UPSTREAM" } else { "DOWNSTREAM" };

        match result {
          Ok(Ok(_)) => info!("Flow {} finished naturally", direction),
          Ok(Err(e)) => error!("Flow {} failed: {}", direction, e),
          Err(e) => error!("Flow {} task panicked: {}", direction, e),
        }
      },
      _ = self.cancellation_token.cancelled() => {
        debug!("Bridge {} received cancellation signal", self.flow_name);
      }
    }

    self.cancellation_token.cancel();

    info!("Shutting down bridge {} connections...", self.flow_name);

    let remote_close = self.remote_conn.close();
    match timeout(Duration::from_secs(3), remote_close).await {
      Ok(Ok(_)) => debug!("Remote connection closed cleanly"),
      Ok(Err(e)) => warn!("Remote connection close error: {}", e),
      Err(_) => warn!("Remote connection close timed out"),
    }

    let local_close = self.local_conn.close();
    let _ = timeout(Duration::from_secs(1), local_close).await;

    Ok(())
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
    let idle_timeout = self.idle_timeout.clone();

    Ok(proxy_flow(
      flow_name,
      cancellation_token,
      local_addr,
      remote_addr,
      local_conn,
      remote_conn,
      client_addr_cache,
      None,
      idle_timeout
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
    let idle_timeout = self.idle_timeout.clone();

    Ok(proxy_flow(
      flow_name,
      cancellation_token,
      remote_addr,
      local_addr,
      remote_conn,
      local_conn,
      None,
      client_addr_cache,
      idle_timeout
    ))
  }
}
