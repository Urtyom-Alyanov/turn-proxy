use std::{any::Any, io::Error as IoError, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use turn::client::Client as StunClient;
use webrtc_util::{Conn, Result as WebRtcResult};

pub struct TurnConn
{
  pub client: StunClient,
  pub relay: Arc<dyn Conn + Send + Sync>,
}

#[async_trait]
impl Conn for TurnConn
{
  fn as_any(&self) -> &(dyn Any + Send + Sync)
  {
    self
  }

  async fn connect(&self, addr: SocketAddr) -> WebRtcResult<()>
  {
    self.relay.connect(addr).await
  }

  async fn recv(&self, buf: &mut [u8]) -> WebRtcResult<usize>
  {
    self.relay.recv(buf).await
  }

  async fn recv_from(&self, buf: &mut [u8])
  -> WebRtcResult<(usize, SocketAddr)>
  {
    self.relay.recv_from(buf).await
  }

  async fn send(&self, buf: &[u8]) -> WebRtcResult<usize>
  {
    self.relay.send(buf).await
  }

  async fn send_to(&self, buf: &[u8], target: SocketAddr)
  -> WebRtcResult<usize>
  {
    self.relay.send_to(buf, target).await
  }

  fn local_addr(&self) -> WebRtcResult<SocketAddr>
  {
    self.relay.local_addr()
  }

  fn remote_addr(&self) -> Option<SocketAddr>
  {
    self.relay.remote_addr()
  }

  async fn close(&self) -> WebRtcResult<()>
  {
    // Закрываем всё по цепочке
    let _ = self.relay.close().await;
    self
      .client
      .close()
      .await
      .map_err(|e| webrtc_util::Error::from(IoError::other(e.to_string())))
  }
}
