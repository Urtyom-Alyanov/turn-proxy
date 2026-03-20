use dtls::config::{Config as DtlsConfig, ExtendedMasterSecretType};
use dtls::crypto::{Certificate, CryptoPrivateKey};
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use tracing::{error, info};

pub fn dtls_configure() -> anyhow::Result<DtlsConfig> {
  // TODO: Реализовать чтение уже готовых сертификатов, что обеспечит более безопасное подключение,
  //       отпечаток (или что-то типа того) которого можно указать в клиенте, если сертификата нет,
  //       то будем генерировать как сейчас.

  info!("Signing certificates...");
  let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
  let params = CertificateParams::default();
  let cert = params.self_signed(&key_pair)?;

  info!("DTLS configuring...");
  let dtls_cert = Certificate {
    certificate: vec![cert.der().to_vec().into()],
    private_key: CryptoPrivateKey::from_key_pair(&key_pair)
      .map_err(|e| error!("DTLS key parsing error: {}", e))
      .unwrap(),
  };
  let dtls_config = DtlsConfig {
    certificates: vec![dtls_cert],
    extended_master_secret: ExtendedMasterSecretType::Request,
    server_name: "localhost".to_string(),
    mtu: 1200,
    ..Default::default()
  };

  Ok(dtls_config)
}
