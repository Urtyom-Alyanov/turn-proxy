use std::{
  net::{IpAddr, SocketAddr},
  sync::Arc,
};

use anyhow::{Context, Result};
use dtls::config::Config as DtlsConfig;
use tokio::net::UdpSocket;
use tracing::info;
use webrtc_util::Conn;

use crate::{
  configuration::configuration::ProviderConfiguration,
  dtls::dtls_configure::dtls_process_handshake,
  proxy_process::{
    setup_and_run_provider::setup_and_run_provider, target_conn::TargetedConn,
  },
};

pub async fn setup_connection(
  thread_id: &str,
  interface_addr: IpAddr,
  provider: &ProviderConfiguration,
  peer_addr: SocketAddr,
  dtls_config: DtlsConfig,
) -> Result<Arc<dyn Conn + Send + Sync>>
{
  let bind_addr = SocketAddr::new(interface_addr, 0);
  // let std_socket = std::net::UdpSocket::bind(bind_addr)?;

  let outbound = UdpSocket::bind(bind_addr).await?;

  info!("UDP socket bound to {}", outbound.local_addr()?);

  let base_conn = Arc::new(outbound) as Arc<dyn Conn + Send + Sync>;

  let remote_conn =
    setup_and_run_provider(interface_addr, provider, base_conn, peer_addr)
      .await?;

  let targeted_conn = Arc::new(TargetedConn {
    inner: remote_conn,
    remote_addr: peer_addr,
  });

  let secure_conn: Arc<dyn Conn + Sync + Send> =
    if provider.using_dtls_obfuscation {
      dtls_process_handshake(thread_id, targeted_conn, dtls_config)
        .await
        .context("Failed to configure DTLS connection")?
    } else {
      targeted_conn
    };

  Ok(secure_conn)
}
