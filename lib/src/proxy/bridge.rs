use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use webrtc_util::Conn;

use crate::proxy::flow::proxy_flow;

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
