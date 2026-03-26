use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use dtls::{
  cipher_suite::CipherSuiteId,
  config::{Config as DtlsConfig, ExtendedMasterSecretType},
  conn::DTLSConn,
  crypto::CryptoPrivateKey,
};
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use tokio::time::timeout;
use tracing::{error, info};
use webrtc_util::Conn;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const FLIGHT_INTERVAL: Duration = Duration::from_millis(2000);

pub fn dtls_configure() -> Result<DtlsConfig>
{
  let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
  info!("Key pair for certificate generated");

  let params = CertificateParams::default();
  let cert = params.self_signed(&key_pair)?;
  info!("Certificate self-signed");

  let dtls_cert = dtls::crypto::Certificate {
    certificate: vec![cert.der().to_vec().into()],
    private_key: CryptoPrivateKey::from_key_pair(&key_pair)
      .map_err(|e| error!("DTLS key parsing error: {}", e))
      .unwrap(),
  };

  let config = DtlsConfig {
    certificates: vec![dtls_cert],
    extended_master_secret: ExtendedMasterSecretType::Request,
    cipher_suites: vec![CipherSuiteId::Tls_Ecdhe_Ecdsa_With_Aes_128_Gcm_Sha256],
    insecure_skip_verify: true,
    server_name: String::new(),
    flight_interval: FLIGHT_INTERVAL,
    mtu: 1280,
    replay_protection_window: 1024,

    ..Default::default()
  };
  info!("DTLS configured");

  Ok(config)
}

pub async fn dtls_process_handshake(
  thread_name: &str,
  conn: Arc<dyn Conn + Send + Sync>,
  config: DtlsConfig,
) -> Result<Arc<DTLSConn>>
{
  let handshake_fut = DTLSConn::new(conn, config, true, None);

  match timeout(HANDSHAKE_TIMEOUT, handshake_fut).await {
    Ok(Ok(dtls_conn)) => {
      info!("[{}] DTLS Handshake completed successfully", thread_name);
      Ok(Arc::new(dtls_conn))
    }
    Ok(Err(e)) => {
      Err(anyhow!("[{}] DTLS handshake failed: {}", thread_name, e))
    }
    Err(_) => Err(anyhow!(
      "[{}] DTLS handshake timed out after 10s - \
      server is not responding or TURN relay is not forwarding",
      thread_name
    )),
  }
}
