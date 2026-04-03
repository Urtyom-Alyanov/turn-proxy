use std::collections::HashMap;
use anyhow::{anyhow,Result};
use reqwest::Client;
use serde_json::Value;
use tracing::info;
use crate::providers::vk::captcha_solve::solve_captcha;

const VK_CLIENT_SECRET: &str = "QbYic1K3lEV5kTGiqlq2";
const VK_CLIENT_ID: &str = "6287487";
const VK_API_VERSION: &str = "5.274";

pub async fn vk_api_request(
  client: &Client,
  method: &str,
  access_token: &str,
  method_body: HashMap<String, String>
) -> Result<Value>
{
  let method_url = format!("https://api.vk.ru/method/{}", method);

  let mut captcha_params: HashMap<String, String> = HashMap::new();
  let max_attempts = 3;

  for attempt in 0..max_attempts {
    let mut body = HashMap::from([
      ("client_id".to_owned(), VK_CLIENT_ID.to_owned()),
      ("access_token".to_owned(), access_token.to_owned()),
      ("v".to_owned(), VK_API_VERSION.to_owned()),
    ]);

    body.extend(captcha_params.clone());
    body.extend(method_body.clone());

    let resp = client.post(&method_url)
      .form(&body)
      .send()
      .await?
      .json::<Value>()
      .await?;

    if let Some(error_object) = resp["error"].as_object() {
      let error_object_clone = error_object.clone();
      captcha_params = solve_captcha(error_object_clone, attempt, max_attempts).await?;

      info!("Captcha solved, retrying request (attempt {})...", attempt + 1);
      continue;
    }

    return Ok(resp["response"].clone())
  }

  Err(anyhow!("Failed to api request after maximum captcha retries"))
}