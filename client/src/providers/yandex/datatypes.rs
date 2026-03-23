use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ConferenceResponse
{
  pub peer_id: String,
  pub room_id: String,
  pub credentials: String,
  pub client_configuration: MediaConfig,
}

#[derive(Deserialize)]
pub struct MediaConfig
{
  pub media_server_url: String,
}

#[derive(Serialize)]
pub struct HelloRequest
{
  pub uid: String,
  pub hello: HelloPayload,
}

#[derive(Serialize)]
pub struct HelloPayload
{
  #[serde(rename = "participantMeta")]
  pub participant_meta: serde_json::Value,
  #[serde(rename = "participantAttributes")]
  pub participant_attributes: serde_json::Value,
  #[serde(rename = "participantId")]
  pub participant_id: String,
  #[serde(rename = "roomId")]
  pub room_id: String,
  #[serde(rename = "serviceName")]
  pub service_name: String,
  pub credentials: String,
  #[serde(rename = "capabilitiesOffer")]
  pub capabilities_offer: serde_json::Value,
  #[serde(rename = "sdkInfo")]
  pub sdk_info: serde_json::Value,
  #[serde(rename = "sdkInitializationId")]
  pub sdk_init_id: String,
}

#[derive(Deserialize)]
pub struct WssResponse
{
  #[serde(rename = "serverHello")]
  pub server_hello: Option<ServerHello>,
}

#[derive(Deserialize)]
pub struct ServerHello
{
  #[serde(rename = "rtcConfiguration")]
  pub rtc_configuration: RtcConfig,
}

#[derive(Deserialize)]
pub struct RtcConfig
{
  #[serde(rename = "iceServers")]
  pub ice_servers: Vec<IceServer>,
}

#[derive(Deserialize)]
pub struct IceServer
{
  pub urls: Vec<String>,
  pub username: Option<String>,
  pub credential: Option<String>,
}
