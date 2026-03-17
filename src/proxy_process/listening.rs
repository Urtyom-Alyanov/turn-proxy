use crate::config::configuration::AppConfig;
use crate::proxy_process::handle_encrypted_udp_connection::handle_encrypted_udp_connection;

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use webrtc_dtls::listener::listen;
use webrtc_dtls::config::{Config as DtlsConfig};
use webrtc_util::conn::Listener;

pub async fn listening(config: AppConfig, dtls_config: DtlsConfig) -> Result<()> {
  let listen_addr: SocketAddr = config.common.listening_on.unwrap().parse()
    .context("'listening-on' is not a valid socket address")?;
  let proxy_addr: SocketAddr = config.common.proxy_into.unwrap().parse()
    .context("'proxy-into' is not a valid socket address")?;

  info!("Listening on: {} UDP", listen_addr);
  info!("Proxying to: {} UDP", proxy_addr);
  let listener = listen(listen_addr, dtls_config).await?;

  let cancel_token = CancellationToken::new();
  let mut cancel_set = JoinSet::new();

  let ct = cancel_token.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();
    info!("Shutdown signal received. Closing connections...");
    ct.cancel();
  });

  info!("Proxy server is up");

  loop {
    tokio::select! {
      _ = cancel_token.cancelled() => break,
      conn_result = listener.accept() => {
        let (conn, remote_addr): (_, _) = match conn_result {
          Ok(res) => res,
          Err(e) => {
            if cancel_token.is_cancelled() { break; }
            warn!("Accept error: {}", e);
            continue;
          }
        };

        let ct_inner = cancel_token.clone();
        let proxy_addr = proxy_addr.clone();

        cancel_set.spawn(async move {
          info!("Connection from: {}", remote_addr);

          let conn_for_shutdown = conn.clone();

          tokio::select! {
            _ = ct_inner.cancelled() => {
              let _ = conn_for_shutdown.close().await;
            }
            res = handle_encrypted_udp_connection(conn, proxy_addr) => {
              if let Err(e) = res {
                warn!("Error handling connection to {}: {}", remote_addr, e);
              }
            }
          }

          info!("Connection closed: {}", remote_addr);
        });
      }
    }
  }

  info!("Waiting for all tasks to finish...");
  let _ = tokio::time::timeout(Duration::from_secs(3), async {
    while let Some(_) = cancel_set.join_next().await {}
  }).await;

  info!("Server stopped.");

  Ok(())
}