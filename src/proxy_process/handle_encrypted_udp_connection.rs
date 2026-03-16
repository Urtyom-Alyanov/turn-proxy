use anyhow::{Result,Context};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use webrtc_util::Conn;

pub async fn handle_encrypted_udp_connection(dtls_conn: Arc<dyn Conn + Send + Sync>, target_addr: SocketAddr) -> Result<()> {
  let target_socket = UdpSocket::bind("0.0.0.0:0").await
    .context("Failed to bind local UDP socket")?;

  debug!("Local socket {} successfully bound", target_socket.local_addr()?);

  if let Err(e) = target_socket.connect(target_addr).await {
    error!("Failed to connect to target addr {}: {:?}", target_addr, e);
    return Err(e).context("Failed to connect to target addr");
  }

  debug!("Successfully connected to target {} from {}", target_addr, target_socket.local_addr()?);

  let socket_arc = Arc::new(target_socket);

  let from_dtls = Arc::clone(&dtls_conn);
  let to_dtls = Arc::clone(&dtls_conn);
  let from_socket = Arc::clone(&socket_arc);
  let to_socket = Arc::clone(&socket_arc);

  let idle_timeout = Duration::from_hours(6);

  let token = tokio_util::sync::CancellationToken::new();
  let t1 = token.clone();
  let t2 = token.clone();

  // client -> proxy -> target
  let client_to_proxy: JoinHandle<Result<()>> = tokio::spawn(async move {
    let mut buf = [0u8; 2048];
    loop {
      match from_dtls.recv(&mut buf).await {
        Ok(n) if n > 0 => {
          debug!("Received {} bytes from {}", n, from_dtls.local_addr()?);
          if n >= buf.len() {
            warn!("Packet from {} is too large for buffer ({})", from_dtls.local_addr().unwrap(), n);
          }
          if let Err(e) = to_socket.send(&buf[..n]).await {
            warn!("Error sending to UDP {} from {}: {}", to_socket.local_addr().unwrap(), from_dtls.local_addr().unwrap(), e);
            break;
          }
          info!("Send {} bytes into {}", n, to_socket.local_addr()?);
        }
        _ => break,
      }
    }

    t1.cancel();
    Ok(())
  });

  // client <- proxy <- target
  let target_to_proxy: JoinHandle<Result<()>> = tokio::spawn(async move {
    let mut buf = [0u8; 2048];

    loop {
      match from_socket.recv(&mut buf).await {
        Ok(n) if n > 0 => {
          debug!("Received {} bytes from {}", n, from_socket.local_addr()?);
          if let Err(e) = to_dtls.send(&buf[..n]).await {
            debug!("Error sending to DTLS: {}", e);
            break;
          }
          info!("Send {} bytes into {}", n, to_dtls.local_addr()?);
        }
        _ => break,
      }
    }

    t2.cancel();
    Ok(())
  });

  let result = tokio::time::timeout(idle_timeout, async {
    tokio::select! {
      _ = client_to_proxy => { debug!("Client task finished"); },
      _ = target_to_proxy => { debug!("Target task finished"); },
    }
  }).await;

  if result.is_err() {
    info!("Connection timed out due to inactivity");
    token.cancel();
  }

  let _ = dtls_conn.close().await;
  Ok(())
}