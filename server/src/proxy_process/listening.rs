use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use dtls::{config::Config as DtlsConfig, listener::listen};
use tokio::{sync::Semaphore, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use webrtc_util::conn::Listener;

use crate::{
  config::configuration::AppConfig,
  proxy_process::handle_encrypted_udp_connection::handle_encrypted_udp_connection,
};

pub async fn listening(config: AppConfig, dtls_config: DtlsConfig)
-> Result<()>
{
  let listen_addr: SocketAddr = config
    .common
    .listening_on
    .unwrap()
    .parse()
    .context("'listening-on' is not a valid socket address")?;
  let proxy_addr: SocketAddr = config
    .common
    .proxy_into
    .unwrap()
    .parse()
    .context("'proxy-into' is not a valid socket address")?;

  info!("Listening on: {} DTLS UDP", listen_addr);
  info!("Proxying to: {} UDP", proxy_addr);
  let listener = listen(listen_addr, dtls_config).await?;

  let cancel_token = CancellationToken::new();
  let mut cancel_set = JoinSet::new();

  let semaphore = Arc::new(Semaphore::new(
    config.common.max_connections.unwrap_or(2000),
  ));

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
      res = cancel_set.join_next(), if !cancel_set.is_empty() => {
        if let Some(Err(e)) = res {
          error!("Task panicked or failed: {:?}", e);
        }
      },
      conn_result = listener.accept() => {
        let (conn, remote_addr): (_, _) = match conn_result {
          Ok(res) => res,
          Err(e) => {
            if cancel_token.is_cancelled() { break; }
            warn!("Accept error: {}", e);
            continue;
          }
        };

        let semaphore_permit = semaphore.clone().try_acquire_owned();

        if let Ok(permit) = semaphore_permit {
          let ct_inner = cancel_token.clone();
          // let proxy_addr = proxy_addr;

          cancel_set.spawn(async move {
            info!("Connection from: {}", remote_addr);

            let _permit = permit;

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
        } else {
          warn!("Max connections reached, dropping connection from {}", remote_addr);
          let _ = conn.close().await;
        }
      }
    }
  }

  info!("Waiting for all tasks to finish...");
  let _ = tokio::time::timeout(Duration::from_secs(3), async {
    while cancel_set.join_next().await.is_some() {}
  })
  .await;

  info!("Server stopped.");

  Ok(())
}
