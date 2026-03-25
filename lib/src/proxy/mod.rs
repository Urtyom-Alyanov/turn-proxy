use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use webrtc_util::Conn;
use crate::proxy::bridge::ProxyBridge;

pub mod bridge;
mod flow;

/// Запуск полностью настроенного моста
pub async fn run_proxy_bridge(
  flow_name: String,
  token: CancellationToken,
  idle_timeout: Option<Duration>,
  first_conn: Arc<dyn Conn + Send + Sync>,
  last_conn: Arc<dyn Conn + Send + Sync>,
  use_cache: bool,
) -> Result<()> {
  let cache_addr = match use_cache {
    true => Some(Arc::new(RwLock::new(None))),
    false => None
  };

  let bridge = ProxyBridge::new(
    flow_name,
    token.clone(),
    first_conn,
    last_conn,
    cache_addr
  );

  let (up, down) = bridge.spawn()?;

  let bridge_future = async {
    tokio::select! {
      res = up => { debug!("Upstream finished: {:?}", res); },
      res = down => { debug!("Downstream finished: {:?}", res); },
    }
  };

  if let Some(t) = idle_timeout {
    if timeout(t, bridge_future).await.is_err() {
      info!("Bridge connection timed out");
    }
  } else {
    bridge_future.await;
  }

  token.cancel();
  Ok(())
}