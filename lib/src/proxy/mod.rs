use std::{sync::Arc, time::Duration};

use anyhow::Result;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
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
) -> Result<()>
{
  let cache_addr = match use_cache {
    true => Some(Arc::new(RwLock::new(None))),
    false => None,
  };

  let bridge = ProxyBridge::new(
    flow_name,
    token,
    first_conn,
    last_conn,
    cache_addr,
    idle_timeout,
  );

  bridge.run().await
}
