use std::fs;
use std::net::SocketAddr;
use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use crate::configuration_file::AppConfig;
use webrtc_dtls::config::{Config as DtlsConfig, ExtendedMasterSecretType};
use webrtc_dtls::listener::listen;
use webrtc_util::Conn;
use webrtc_util::conn::Listener;

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
async fn main() -> anyhow::Result<()> {
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

  println!("[LOG] Listening on: {} UDP", listen_addr);
  println!("[LOG] Proxying to: {} UDP", proxy_addr);
  let listener = listen(listen_addr, dtls_config).await?;

  let shutdown_notify = Arc::new(tokio::sync::Notify::new());
  let sn_clone = shutdown_notify.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();
    println!("Terminating...");
    sn_clone.notify_waiters();
  });

  println!("[SUCCESS] Proxy server is up");

  loop {
    tokio::select! {
      _ = shutdown_notify.notified() => break,
      conn_result = listener.accept() => {
        let (conn, remote_addr) = match conn_result {
          Ok(res) => res,
          Err(e) => {
            eprintln!("[WARN] Accept error: {}", e);
            continue;
          }
        };

        let proxy_addr = proxy_addr.clone();
        tokio::spawn(async move {
          println!("[LOG] connection from: {}", remote_addr);
          if let Err(e) = handle_connection(conn, proxy_addr).await {
            eprintln!("[WARN] error handling connection to {}: {}", remote_addr, e);
          }
          println!("[LOG] connection closed: {}", remote_addr);
        });
      }
    }
  }

  Ok(())
}

async fn dtls_configure() -> Result<DtlsConfig> {
  println!("[LOG] Signing certificates...");
  let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
  let mut params = CertificateParams::default();
  let cert = params.self_signed(&key_pair)?;

  println!("[LOG] DTLS configuring...");
  let dtls_cert = webrtc_dtls::crypto::Certificate {
    certificate: vec![cert.der().to_vec().into()],
    private_key: webrtc_dtls::crypto::CryptoPrivateKey::from_key_pair(
      &key_pair).map_err(|e| panic!("[LOG] DTLS key parsing error: {}", e)).unwrap(),
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
    .context("[ERROR] Failed to bind local UDP socket")?;

  target_socket.connect(target_addr).await
    .context("[ERROR] Failed to connect to target addr")?;

  let mut buf_in = [0u8; 1280];
  let mut buf_out = [0u8; 1280];
  let idle_timeout = Duration::from_hours(6);

  loop {
    tokio::select! {
      // read from DTLS (client -> proxy)
      res = dtls_conn.recv(&mut buf_in) => {
        match res {
          Ok(bytes) if bytes > 0 => {
            target_socket.send(&buf_in[..bytes]).await?;
          },
          Ok(_) => break,
          Err(e) => {
            return Err(anyhow::anyhow!("[WARN] DTLS error: {}", e));
          }
        }
      }
      // read from UDP (target -> proxy)
      res = target_socket.recv(&mut buf_out) => {
          let n = res?;
          dtls_conn.send(&buf_out[..n]).await.context("[ERROR] DTLS write error")?;
      }

      _ = tokio::time::sleep(idle_timeout) => {
          println!("[LOG] Connection idle timeout reached (no activity)");
          break;
      }
    }
  }

  let _ = dtls_conn.close().await;
  Ok(())
}