mod datatypes;

use std::{net::IpAddr, sync::LazyLock};

use anyhow::{Context, Result, anyhow};
use futures_util::{sink::SinkExt, stream::StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;
use tokio_tungstenite::{
  connect_async,
  tungstenite::{
    handshake::client::Request as TungsteniteRequest, protocol::Message,
  },
};
use uuid::Uuid;

use crate::{
  inbound::{USER_AGENT, create_inbound_client},
  providers::yandex::datatypes::{
    ConferenceResponse, HelloPayload, HelloRequest, WssResponse,
  },
  proxy_process::turn_configure::TurnCredentials,
};

const YANDEX_REALM: &str = "yandex";
const YANDEX_SERVICE_NAME: &str = "telemost";
static YANDEX_SDK_INFO: LazyLock<serde_json::Value> = LazyLock::new(|| {
  json!({
    "implementation": "browser", "version": "5.15.0",
    "userAgent": USER_AGENT, "hwConcurrency": 4
  })
});
static YANDEX_CAPABILITIES_OFFER: LazyLock<serde_json::Value> = LazyLock::new(
  || {
    json!({
      "offerAnswerMode": ["SEPARATE"],
      "initialSubscriberOffer": ["ON_HELLO"],
      "slotsMode": ["FROM_CONTROLLER"],
      "simulcastMode": ["DISABLED"],
      "selfVadStatus": ["FROM_SERVER"],
      "dataChannelSharing": ["TO_RTP"],
      "videoEncoderConfig": ["NO_CONFIG"],
      "dataChannelVideoCodec": ["VP8"],
      "bandwidthLimitationReason": ["BANDWIDTH_REASON_DISABLED"],
      "sdkDefaultDeviceManagement": ["SDK_DEFAULT_DEVICE_MANAGEMENT_DISABLED"],
      "joinOrderLayout": ["JOIN_ORDER_LAYOUT_DISABLED"],
      "pinLayout": ["PIN_LAYOUT_DISABLED"],
      "sendSelfViewVideoSlot": ["SEND_SELF_VIEW_VIDEO_SLOT_DISABLED"],
      "serverLayoutTransition": ["SERVER_LAYOUT_TRANSITION_DISABLED"],
      "sdkPublisherOptimizeBitrate": ["SDK_PUBLISHER_OPTIMIZE_BITRATE_DISABLED"],
      "sdkNetworkLostDetection": ["SDK_NETWORK_LOST_DETECTION_DISABLED"],
      "sdkNetworkPathMonitor": ["SDK_NETWORK_PATH_MONITOR_DISABLED"],
      "publisherVp9": ["PUBLISH_VP9_DISABLED"],
      "svcMode": ["SVC_MODE_DISABLED"],
      "subscriberOfferAsyncAck": ["SUBSCRIBER_OFFER_ASYNC_ACK_DISABLED"],
      "svcModes": ["FALSE"], "reportTelemetryModes": ["TRUE"], "keepDefaultDevicesModes": ["TRUE"]
    })
  },
);

pub fn get_yandex_call_id_from_link(link: &str) -> Result<&str>
{
  link
    .trim()
    .split("j/")
    .last()
    .ok_or(anyhow!("Invalid link"))
}

pub async fn get_yandex_telebridge_turn_credentials(
  interface: IpAddr,
  call_id: &str,
  with_name: Option<String>,
) -> Result<TurnCredentials>
{
  let client = create_inbound_client(interface).await?;
  let endpoint = format!(
    "https://cloud-api.yandex.ru/telemost_front/v2/telemost/conferences/https%3A%2F%2Ftelemost.yandex.ru%2Fj%2F{}/connection?next_gen_media_platform_allowed=false",
    call_id
  );

  let mut headers = HeaderMap::new();
  headers.insert("User-Agent", HeaderValue::from_static(USER_AGENT));
  headers.insert(
    "Origin",
    HeaderValue::from_static("https://telemost.yandex.ru"),
  );
  headers.insert(
    "Referer",
    HeaderValue::from_static("https://telemost.yandex.ru/"),
  );
  headers.insert(
    "Client-Instance-Id",
    HeaderValue::from_str(&Uuid::new_v4().to_string())?,
  );

  let conf_resp = client
    .get(&endpoint)
    .headers(headers.clone())
    .send()
    .await?
    .json::<ConferenceResponse>()
    .await?;

  let name = with_name.unwrap_or("Гость".to_owned());

  let ws_request =
    ws_request_builder(&conf_resp.client_configuration.media_server_url)?;
  let (mut ws_stream, _) = connect_async(ws_request)
    .await
    .map_err(|e| anyhow!("WS connect error: {}", e))?;

  let hello = HelloRequest {
    uid: Uuid::new_v4().to_string(),
    hello: HelloPayload {
      participant_meta: json!({
        "name": &name, "role": "SPEAKER", "description": "",
        "sendAudio": false, "sendVideo": false
      }),
      participant_attributes: json!({ "name": &name, "role": "SPEAKER", "description": "" }),
      participant_id: conf_resp.peer_id,
      room_id: conf_resp.room_id,
      service_name: YANDEX_SERVICE_NAME.to_string(),
      credentials: conf_resp.credentials,
      sdk_init_id: Uuid::new_v4().to_string(),
      sdk_info: YANDEX_SDK_INFO.clone(),
      capabilities_offer: YANDEX_CAPABILITIES_OFFER.clone(),
    },
  };

  let msg = serde_json::to_string(&hello)?;
  ws_stream.send(Message::Text(msg.into())).await?;

  while let Some(Ok(Message::Text(text))) = ws_stream.next().await {
    if let Ok(resp) = serde_json::from_str::<WssResponse>(&text)
      && let Some(hello) = resp.server_hello
    {
      for server in hello.rtc_configuration.ice_servers {
        for url in server.urls {
          if (url.starts_with("turn:") || url.starts_with("turns:"))
            && !url.contains("transport=tcp")
          {
            let clean_addr = url
              .trim_start_matches("turn:")
              .trim_start_matches("turns:")
              .split('?')
              .next()
              .unwrap_or("")
              .to_string();

            return Ok(TurnCredentials {
              username: server.username.context("No WS username")?,
              password: server.credential.context("No WS password")?,
              realm: YANDEX_REALM.to_string(),
              turn_addr: clean_addr.clone(),
              stun_addr: Some(clean_addr),
            });
          }
        }
      }
    }
  }

  Err(anyhow!("Failed to extract TURN creds from Yandex WS"))
}

fn ws_request_builder(url: &str) -> Result<TungsteniteRequest>
{
  let request = TungsteniteRequest::builder()
    .uri(url)
    .header("Origin", "https://telemost.yandex.ru")
    .header("User-Agent", USER_AGENT)
    .body(())?;
  Ok(request)
}
