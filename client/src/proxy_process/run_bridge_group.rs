use std::sync::Arc;

use anyhow::Result;
use futures_util::future::select_all;
use tokio::{net::UdpSocket, sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use turn_proxy_lib::proxy::bridge::ProxyBridge;
use webrtc_util::Conn;

use crate::proxy_process::target_conn::TargetedConn;

pub async fn run_bridge_thread(
  thread_num: usize,
  listen_conn: Arc<UdpSocket>,
  remote_conn: Arc<dyn Conn + Send + Sync>,
  token: CancellationToken,
) -> Result<()>
{
  let mut handles: Vec<JoinHandle<Result<()>>> = vec![];
  let local_conn = Arc::new(TargetedConn {
    inner: listen_conn.clone(),
    remote_addr: listen_conn.local_addr()?,
  });

  let thread_id = format!("T{}", thread_num);
  let bridge = ProxyBridge::new(
    thread_id,
    token.clone(),
    local_conn,
    remote_conn,
    Some(Arc::new(RwLock::new(None))),
  );

  let (up, down) = bridge.spawn()?;

  // let up = bridge
  //   .run_upstream(local_conn.clone(), remote_conn.clone())
  //   .await?;
  // let down = bridge
  //   .run_downstream(local_conn.clone(), remote_conn.clone())
  //   .await?;

  handles.push(up);
  handles.push(down);

  if let Some(result) = select_all(handles).await.0.ok() {
    result?;
  }

  token.cancel();
  Ok(())
}
