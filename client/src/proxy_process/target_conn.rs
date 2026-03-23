use std::{any::Any, net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use webrtc_util::{Conn, Result};

/// Высокоуровневая абстракция для TURN-соединения (при `send` отправляет на
/// `remote_addr`), но также может быть использована и с обычным UDP-соединением
/// (прямым, то бишь). Используется при DTLS-рукопожатии (он использует `send` и
/// `recv`)
pub struct TargetedConn
{
  pub inner: Arc<dyn Conn + Send + Sync>,
  pub remote_addr: SocketAddr,
}

#[async_trait]
impl Conn for TargetedConn
{
  fn as_any(&self) -> &(dyn Any + Send + Sync)
  {
    self
  }

  async fn connect(&self, _addr: SocketAddr) -> Result<()>
  {
    Ok(())
  }

  async fn recv(&self, buf: &mut [u8]) -> Result<usize>
  {
    let (n, _) = self.inner.recv_from(buf).await?;
    Ok(n)
  }

  async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)>
  {
    Ok(self.inner.recv_from(buf).await?)
  }

  async fn send(&self, buf: &[u8]) -> Result<usize>
  {
    Ok(self.inner.send_to(buf, self.remote_addr).await?)
  }

  async fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize>
  {
    Ok(self.inner.send_to(buf, target).await?)
  }

  fn local_addr(&self) -> Result<SocketAddr>
  {
    Ok(self.inner.local_addr()?)
  }

  fn remote_addr(&self) -> Option<SocketAddr>
  {
    Some(self.remote_addr)
  }

  async fn close(&self) -> Result<()>
  {
    Ok(self.inner.close().await?)
  }
}
