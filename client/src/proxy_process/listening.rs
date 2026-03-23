use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use dtls::config::Config as DtlsConfig;
use tokio::{net::UdpSocket, task::JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use webrtc_util::Conn;

use crate::{
  configuration::configuration::{AppConfiguration, ProviderConfiguration},
  dtls::dtls_configure::dtls_process_handshake,
  proxy_process::{
    run_bridge_group::run_bridge_thread, setup_and_run_provider::setup_and_run_provider,
    target_conn::TargetedConn,
  },
};

pub async fn listening(config: AppConfiguration, dtls_config: DtlsConfig) -> Result<()>
{
  let listen_addr: SocketAddr = config
    .common
    .listening_on
    .parse()
    .context("'listening-on' is not a valid socket address")?;
  let peer_addr: SocketAddr = config
    .common
    .peer_addr
    .parse()
    .context("'proxy-into' is not a valid socket address")?;

  info!("Listening on: {} UDP", listen_addr);
  info!("Proxying to: {} DTLS UDP", peer_addr);

  let listen_socket: Arc<UdpSocket> = Arc::new(UdpSocket::bind(listen_addr).await?);

  let cancel_token = CancellationToken::new();

  let ct = cancel_token.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();
    info!("Shutdown signal received. Closing connections...");
    ct.cancel();
  });

  info!("Sorting providers with priorities...");
  let mut providers = config.providers.clone();
  providers.sort_by_key(|p| p.priority.unwrap_or(u32::MAX));

  loop {
    if cancel_token.is_cancelled() {
      break;
    }

    for provider in &providers {
      info!(
        "Trying provider with priority {:?}",
        provider.priority.unwrap_or(1)
      );

      let thread_count = provider.threads.unwrap_or(1);
      let mut handles = JoinSet::new();

      for thread_id in 0..thread_count {
        let p_clone = provider.clone();
        let l_clone = listen_socket.clone();
        let p_addr = peer_addr;
        let t_token = cancel_token.child_token();
        let dtls_cert_copy = dtls_config.clone();
        let write_addr = config.common.write_addr.clone();

        handles.spawn(async move {
          let conn = setup_connection(
            format!("T{}", thread_id).as_str(),
            &p_clone,
            p_addr,
            dtls_cert_copy,
            write_addr,
          )
          .await?;

          run_bridge_thread(thread_id, l_clone, conn, t_token).await
        });
      }

      let should_try_next = tokio::select! {
        _ = cancel_token.cancelled() => {
          info!("Terminating all threads...");
          false
        }
        Some(res) = handles.join_next() => {
          match res {
            Ok(Ok(_)) => warn!("A thread finished successfully. Switching provider..."),
            Ok(Err(e)) => error!("Thread error: {}. Switching...", e),
            Err(e) => error!("Thread panicked: {}", e),
          }
          true
        }
      };

      handles.shutdown().await;

      if !should_try_next || cancel_token.is_cancelled() {
        break;
      }
    }

    if !cancel_token.is_cancelled() {
      warn!("All providers failed or finished. Retrying in 5s...");
      tokio::select! {
        _ = cancel_token.cancelled() => break,
        _ = tokio::time::sleep(Duration::from_secs(5)) => {}
      }
    }
  }

  info!("Terminating...");
  // let _ = tokio::time::timeout(Duration::from_secs(3), async {
  //   while let Some(_) = cancel_set.join_next().await {}
  // }).await;

  Ok(())
}

async fn setup_connection(
  thread_id: &str,
  provider: &ProviderConfiguration,
  peer_addr: SocketAddr,
  dtls_config: DtlsConfig,
  write_addr: Option<bool>,
) -> Result<Arc<dyn Conn + Send + Sync>>
{
  let outbound = UdpSocket::bind("0.0.0.0:0").await?;

  let base_conn = Arc::new(outbound) as Arc<dyn Conn + Send + Sync>;

  let remote_conn = setup_and_run_provider(provider, base_conn, peer_addr).await?;

  if let Ok(proxy_addr) = remote_conn.local_addr() {
    if write_addr.unwrap_or(false) {
      println!("{}", proxy_addr.ip());
    }
  };

  let targeted_conn = Arc::new(TargetedConn {
    inner: remote_conn,
    remote_addr: peer_addr,
  });

  let secure_conn: Arc<dyn Conn + Sync + Send> = if provider.using_dtls_obfuscation {
    dtls_process_handshake(thread_id, targeted_conn, dtls_config)
      .await
      .context("Failed to configure DTLS connection")?
  } else {
    targeted_conn
  };

  Ok(secure_conn)
}
