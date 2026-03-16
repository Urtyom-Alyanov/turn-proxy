use std::fs;
use std::net::SocketAddr;
use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use tracing::{info, error, warn, debug};
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use crate::configuration_file::AppConfig;
use webrtc_dtls::config::{Config as DtlsConfig, ExtendedMasterSecretType};
use webrtc_dtls::listener::listen;
use webrtc_util::Conn;
use webrtc_util::conn::Listener;
use tokio_util::sync::CancellationToken;
use tokio::task::{JoinSet};
use tracing_subscriber::{fmt,prelude::*,EnvFilter};

pub mod configuration_file;

#[derive(Parser, Debug)]
struct Args {
  #[arg(long, default_value = "0.0.0.0:56000")]
  listening_on: Option<String>,

  #[arg(long)]
  proxy_into: Option<String>,

  #[arg(long)]
  no_config: bool,

  #[arg(long, default_value = "/etc/turn-proxy/server/config.toml")]
  config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
  let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

  let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info"));

  tracing_subscriber::registry()
    .with(filter)
    .with(fmt::layer().with_writer(non_blocking))
    .init();

  let args = Args::parse();

  let mut config = if !args.no_config {
    let content = fs::read_to_string(&args.config)
      .with_context(|| format!("[ERROR] read configuration file error: {}", args.config))?;
    toml::from_str::<AppConfig>(&content)
      .context(format!("[ERROR] TOML configuration parse error (path: {})", args.config))?
  } else {
    AppConfig::default()
  };

  let final_listen = args.listening_on
    .or(config.common.listening_on)
    .unwrap_or_else(|| "0.0.0.0:56000".to_string());

  let final_proxy = args.proxy_into
    .or(config.common.proxy_into)
    .context("[ERROR] proxy_into address is missing")?;

  let listen_addr: SocketAddr = final_listen.parse()
    .context("[ERROR] 'listening_on' is not a valid socket address")?;
  let proxy_addr: SocketAddr = final_proxy.parse()
    .context("[ERROR] 'proxy_into' is not a valid socket address")?;

  let dtls_config = dtls_configure().await?;

  info!("Listening on: {} UDP", listen_addr);
  info!("Proxying to: {} UDP", proxy_addr);
  let listener = listen(listen_addr, dtls_config).await?;

  let cancel_token = CancellationToken::new();
  let mut cancel_set = JoinSet::new();

  let shutdown_notify = Arc::new(tokio::sync::Notify::new());
  let sn_clone = shutdown_notify.clone();
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
        let (conn, remote_addr) = match conn_result {
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
            res = handle_connection(conn, proxy_addr) => {
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

async fn dtls_configure() -> Result<DtlsConfig> {
  info!("Signing certificates...");
  let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
  let mut params = CertificateParams::default();
  let cert = params.self_signed(&key_pair)?;

  info!("DTLS configuring...");
  let dtls_cert = webrtc_dtls::crypto::Certificate {
    certificate: vec![cert.der().to_vec().into()],
    private_key: webrtc_dtls::crypto::CryptoPrivateKey::from_key_pair(
      &key_pair).map_err(|e| error!("DTLS key parsing error: {}", e)).unwrap(),
  };
  let dtls_config = DtlsConfig {
    certificates: vec![dtls_cert],
    extended_master_secret: ExtendedMasterSecretType::Request,
    ..Default::default()
  };

  Ok(dtls_config)
}

async fn handle_connection(dtls_conn: Arc<dyn Conn + Send + Sync>, target_addr: SocketAddr) -> Result<()> {
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
  let client_to_proxy: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
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
  let target_to_proxy: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
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