use regex::Regex;
use reqwest::Client;
use anyhow::{Result,anyhow};
use serde_json::Value;

use crate::providers::vk::captcha_solve::redirect_url::{Challenge, ChallengeMeta, extract_session_token, human_emulating::create_human_metrics, proof_of_work::PowChallenge};

fn extract_pow_challenge(html: &str) -> Result<PowChallenge>
{
  let re_input = Regex::new(r#"const\s+powInput\s*=\s*"([^"]+)""#)?;
  let input = re_input
    .captures(&html)
    .and_then(|cap| cap.get(1))
    .map(|m| m.as_str().to_string())
    .ok_or(anyhow!("powInput not found"))?;

  let re_diff = Regex::new(r"startsWith\('0'\.repeat\((\d+)\)\)")?;
  let difficulty = re_diff
    .captures(&html)
    .and_then(|cap| cap.get(1))
    .and_then(|m| m.as_str().parse().ok())
    .unwrap_or(2);

  Ok(PowChallenge { input, difficulty })
}

fn extract_slider_challenge(html: &str) -> Result<Option<String>>
{
  let re_captcha_settings= Regex::new(r"(?s)window\.init\s*=\s*(\{.*?\});")?;

  let mut slider_image_id: Option<String> = None;

  if let Some(captcha_settings_string) = re_captcha_settings.captures(&html) {
    let json_str = &captcha_settings_string[1];

    if let Ok(settings_root) = serde_json::from_str::<Value>(json_str) {

      let captcha_type = settings_root["data"]["show_captcha_type"]
        .as_str().unwrap();

      let captcha_settings = settings_root["data"]["captcha_settings"]
        .as_array();

      let captcha_types_settings = captcha_settings.and_then(
        |settings_array| settings_array.iter().find(|item| item["type"] == captcha_type)
      ).and_then(|slider_obj| slider_obj["settings"].as_str());

      if captcha_type == "slider" {
        slider_image_id = match captcha_types_settings {
          Some(string) => Some(string.to_owned()),
          None => None
        };
      }
    }
  }

  Ok(slider_image_id)
}

// Извлекает из HTML страницы данные для PoW задачи
pub async fn fetch_challenge(
  client: &Client,
  redirect_url: &str,
  access_token: Option<&str>
) -> Result<Challenge>
{
  let session_token = extract_session_token(redirect_url).ok_or(anyhow!("`session_token` has not defined"))?;

  let metrics = create_human_metrics(None);

  let meta = ChallengeMeta {
    session_token,
    access_token: access_token.unwrap_or("").to_owned(),
    metrics,
    redirect_url: redirect_url.to_owned()
  };

  let html = client.get(redirect_url).send().await?.text().await?;

  let proof_of_work = extract_pow_challenge(&html)?;
  let slider_settings = extract_slider_challenge(&html)?;

  Ok(
    Challenge { meta, proof_of_work, slider_settings }
  )
}
