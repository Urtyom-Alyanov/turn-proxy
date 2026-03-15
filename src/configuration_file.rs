use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct AppConfig {
  #[serde(default)]
  pub common: CommonConfig,
}

#[derive(Deserialize, Default)]
pub struct CommonConfig {
  pub listening_on: Option<String>,
  pub proxy_into: Option<String>,
}