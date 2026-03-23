use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use webrtc_util::Conn;

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
