use std::{
  net::{IpAddr, SocketAddr},
  sync::Arc,
  time::Duration,
};

use anyhow::{Result, anyhow};
use tracing::{debug, info, warn};
use webrtc_util::Conn;

use crate::{
  configuration::configuration::{
    DefaultProvider, ProviderConfiguration, ProviderDetails,
  },
  inbound::user_agent::get_random_user_agent,
  providers::{
    vk::{get_vk_call_id_from_link, get_vk_calls_turn_credentials},
    yandex::{
      get_yandex_call_id_from_link, get_yandex_telebridge_turn_credentials,
    },
  },
  proxy_process::turn_configure::{TurnCredentials, turn_configure},
};

pub async fn setup_and_run_provider(
  interface: IpAddr,
  provider: &ProviderConfiguration,
  connection: Arc<dyn Conn + Send + Sync>,
  peer_addr: SocketAddr,
) -> Result<Arc<dyn Conn + Send + Sync>>
{
  match &provider.details {
    ProviderDetails::Direct => {
      info!("Try to direct connection with server...");
      connection.connect(peer_addr).await?;
      Ok(connection)
    }
    _ => {
      info!("Try to connection via TURN...");

      let creds = fetch_creds(interface, &provider.details).await?;
      let turn = turn_configure(connection, creds).await?;

      info!("TURN connection established successfully");
      info!("Warming up TURN channel to {}...", peer_addr);

      match turn.send_to(b"ping", peer_addr).await {
        Ok(_) => debug!("Send_to called"),
        Err(e) => warn!("Send_to error: {}", e),
      };
      tokio::time::sleep(Duration::from_millis(200)).await;

      info!("Connection via TURN Relay established successfully");

      Ok(turn as Arc<dyn Conn + Send + Sync>)
    }
  }
}

async fn fetch_creds(
  interface: IpAddr,
  details: &ProviderDetails,
) -> Result<TurnCredentials>
{
  match details {
    ProviderDetails::Default { kind, link } => {
      let call_id = get_call_id_from_link(kind, link)?;

      match kind {
        DefaultProvider::VkCalls => {
          get_vk_calls_turn_credentials(interface, call_id.to_owned(), None)
            .await
        }
        DefaultProvider::YandexTelemost => {
          let user_agent = get_random_user_agent();

          get_yandex_telebridge_turn_credentials(
            interface,
            call_id,
            &user_agent.value,
            None,
          )
          .await
        }
      }
    }
    ProviderDetails::Custom {
      realm,
      password,
      username,
      stun_address,
      turn_address,
    } => Ok(TurnCredentials {
      password: password.to_owned(),
      realm: realm.to_owned(),
      username: username.to_owned(),
      turn_addr: turn_address.to_owned(),
      stun_addr: stun_address.to_owned().into(),
    }),
    ProviderDetails::Direct => Err(anyhow!(
      "Direct provider does not require TURN credentials fetching"
    )),
  }
}

fn get_call_id_from_link<'a>(
  kind: &DefaultProvider,
  link: &'a str,
) -> Result<&'a str>
{
  match kind {
    DefaultProvider::VkCalls => get_vk_call_id_from_link(link),
    DefaultProvider::YandexTelemost => get_yandex_call_id_from_link(link),
  }
}
