use std::{
  any::Any, net::SocketAddr, sync::{
    Arc, atomic::{
      AtomicUsize,
      Ordering
    }
  }
};

use futures_util::future::{BoxFuture, join_all};
use tokio::{sync::{Mutex, mpsc}, time::sleep};
use tokio_util::sync::CancellationToken;
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};
use webrtc_util::Conn;
use async_trait::async_trait;
use webrtc_util::{Result as WebRtcResult, Error as WebRtcError};
use bytes::{Bytes};

use crate::UDP_MTU;
pub const CHANNEL_BUF: usize = 1024;

const MIN_RETRY_DELAY_SECS: u64 = 5;
const MAX_RETRY_DELAY_SECS: u64 = 30;

const CONNECTION_CLOSING_DELAY: u64 = 5;

type ConnFactory = Arc<
  dyn Fn() -> BoxFuture<'static, WebRtcResult<Arc<dyn Conn + Send + Sync>>> 
  + Send + Sync
>;

/// Соединение, состоящие из `n`-нного количества соединенией (потоков), при отправке выбирает соединение (поток) через алгоритм round-robin
/// 
/// Реализует трейт Conn из webrtc-rs
pub struct BalancedConn {
  connections: Vec<RwLock<Arc<dyn Conn + Sync + Send>>>,
  send_index: AtomicUsize,
  recv_queue: Mutex<mpsc::Receiver<(Bytes, SocketAddr)>>,
  cancel_token: CancellationToken,
}

impl BalancedConn {
  /// Создаёт сбалансированное соединение, начинает слушать каждый поток и кладёт результат слушанья в очередь `recv_queue`
  pub async fn new(
    count: usize, 
    factory: ConnFactory,
    cancel_token: CancellationToken
  ) -> WebRtcResult<Arc<Self>>
  {
    if count <= 0 {
      panic!("Connections list cannot be empty");
    }

    let mut connections: Vec<RwLock<Arc<dyn Conn + Sync + Send>>> = Vec::with_capacity(count);
    let (sender, receiver) = mpsc::channel::<(Bytes, SocketAddr)>(CHANNEL_BUF);
    let ct = cancel_token.child_token();

    for _ in 0..count {
      let conn = factory().await?;
      let shared_conn = RwLock::new(conn);
      connections.push(shared_conn);
    }

    let res = Arc::new(Self {
      cancel_token: ct,
      connections,
      recv_queue: Mutex::new(receiver),
      send_index: AtomicUsize::new(0)
    });

    for idx in 0..count {
      let this = res.clone();
      let sender_clone = sender.clone();
      let factory_clone = factory.clone();
      let ct_worker = res.cancel_token.child_token();

      tokio::spawn(async move {
        this.worker_conn(idx, factory_clone, sender_clone, ct_worker).await;
      });
    }

    Ok(res)
  }

  async fn worker_conn(
    &self,
    idx: usize,
    factory: ConnFactory,
    sender: mpsc::Sender<(Bytes, SocketAddr)>,
    ct: CancellationToken
  ) {
    let mut buf = [0u8; UDP_MTU];
    let mut retry_delay = std::time::Duration::from_secs(MIN_RETRY_DELAY_SECS);

    loop {
      let conn = {
        let lock = self.connections[idx].read();
        lock.clone()
      };

      tokio::select! {
        _ = ct.cancelled() => break,
        res = conn.recv_from(&mut buf) => {
          match res {
            Ok((n, src)) => {
              let data = Bytes::copy_from_slice(&buf[..n]);
              if sender.send((data, src)).await.is_err() { break; }
              retry_delay = std::time::Duration::from_secs(MIN_RETRY_DELAY_SECS);
            },
            Err(e) => {
              warn!(index = idx, "Flow error: {:?}. Reconnecting ({}s)...", e, retry_delay.as_secs());

              sleep(retry_delay).await;

              match factory().await {
                Ok(new_conn) => {
                  info!(index = idx, "Reconnected successfully");
                  let mut lock = self.connections[idx].write();
                  *lock = new_conn;
                  retry_delay = std::time::Duration::from_secs(MIN_RETRY_DELAY_SECS);
                }
                Err(re_err) => {
                  error!(index = idx, "Reconnect failed: {:?}", re_err);
                  retry_delay = std::cmp::min(retry_delay * 2, std::time::Duration::from_secs(MAX_RETRY_DELAY_SECS));
                }
              }
            }
          }
        }
      }
    }
  }
}

#[async_trait]
impl Conn for BalancedConn {
  fn as_any(&self) -> &(dyn Any + Send + Sync) { self }

  fn local_addr(&self) -> WebRtcResult<SocketAddr> { self.connections[0].read().local_addr() }
  fn remote_addr(&self) -> Option<SocketAddr> { self.connections[0].read().remote_addr() }

  async fn connect(&self, addr: SocketAddr) -> WebRtcResult<()>
  {
    let current_connections: Vec<_> = self.connections
      .iter()
      .map(|lock| lock.read().clone())
      .collect();

    let futures = current_connections.iter().map(|c| c.connect(addr));
    let results = join_all(futures).await;

    for res in results {
      res?;
    }

    Ok(())
  }

  async fn close(&self) -> WebRtcResult<()> {
    self.cancel_token.cancel();

    let current_connections: Vec<_> = self.connections
      .iter()
      .map(|lock| lock.read().clone())
      .collect();
    let futures = current_connections.iter().map(|c| c.close());
    
    match tokio::time::timeout(
        std::time::Duration::from_secs(CONNECTION_CLOSING_DELAY), 
        join_all(futures)
    ).await {
      Ok(results) => {
        for res in results {
          if let Err(e) = res {
            error!("Error while closing sub-connection: {:?}", e);
          }
        }
      },
      Err(_) => {
        warn!("Close operation timed out after {} seconds", CONNECTION_CLOSING_DELAY);
      }
    }

    Ok(())
  }

  async fn send(&self, buf: &[u8]) -> WebRtcResult<usize> {
    let idx = self.send_index.fetch_add(1, Ordering::Relaxed) % self.connections.len();

    let conn = {
      let lock = self.connections[idx].read();
      lock.clone()
    };

    conn.send(buf).await
  }

  async fn send_to(&self, buf: &[u8], target: SocketAddr) -> WebRtcResult<usize> {
    let idx = self.send_index.fetch_add(1, Ordering::Relaxed) % self.connections.len();

    let conn = {
      let lock = self.connections[idx].read();
      lock.clone()
    };

    conn.send_to(buf, target).await
  }

  async fn recv_from(&self, buf: &mut [u8]) -> WebRtcResult<(usize, SocketAddr)> {
    let mut queue = self.recv_queue.lock().await;

    match queue.recv().await {
      Some((data, addr)) => {
        let n = data.len().min(buf.len());
        buf[..n].copy_from_slice(&data[..n]);

        if n < data.len() {
          debug!("Provided buffer is smaller than received packet; data truncated");
        }

        Ok((n, addr))
      },
      None => Err(WebRtcError::ErrClosedListener)
    }
  }

  async fn recv(&self, buf: &mut [u8]) -> WebRtcResult<usize> {
    let (n, _) = self.recv_from(buf).await?;
    Ok(n)
  }
}

impl Drop for BalancedConn {
  fn drop(&mut self) {
    self.cancel_token.cancel();
  }
}