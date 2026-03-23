use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, info};
use turn::client::{Client as StunClient, ClientConfig as TurnClientConfig};
use webrtc_util::Conn;

use crate::proxy_process::turn_connection::TurnConn;

pub struct TurnCredentials
{
  pub username: String,
  pub password: String,
  pub realm: String,
  pub turn_addr: String,
  pub stun_addr: Option<String>,
}

/// Настройка подключения к TURN-серверу с полученными учётными данными от
/// поставщика
pub async fn turn_configure(
  conn: Arc<dyn Conn + Send + Sync>,
  credentials: TurnCredentials,
) -> Result<Arc<TurnConn>>
{
  debug!("Setting up connection with {}...", &credentials.turn_addr);

  let client_config = TurnClientConfig {
    stun_serv_addr: credentials
      .stun_addr
      .unwrap_or(credentials.turn_addr.clone()),
    turn_serv_addr: credentials.turn_addr,
    username: credentials.username,
    password: credentials.password,
    realm: credentials.realm,
    conn,
    rto_in_ms: 100,
    vnet: None,
    software: String::new(),
  };

  let client = StunClient::new(client_config).await?;

  client.listen().await?;

  debug!("Connected to TURN server.");

  let relay_conn = client.allocate().await?;

  info!(
    "Relay connection at {} allocated!",
    relay_conn.local_addr()?
  );

  Ok(Arc::new(TurnConn {
    client,
    relay: Arc::new(relay_conn) as Arc<dyn Conn + Sync + Send>,
  }))
}
