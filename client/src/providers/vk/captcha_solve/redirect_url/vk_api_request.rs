use std::collections::HashMap;

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::Value;

/// Внутренняя функция для вызовов VK API
pub async fn method_call(
  client: &Client,
  method: &str,
  method_body: HashMap<String, String>,
) -> Result<Value>
{
  let method_url = format!("https://api.vk.ru/method/{}", method);

  let mut body = HashMap::from([("v".to_owned(), "5.131".to_owned())]);

  body.extend(method_body);

  let response = client
    .post(method_url)
    .header("Origin", "https://vk.ru")
    .header("Referer", "https://vk.ru/")
    .form(&body)
    .send()
    .await?
    .json::<Value>()
    .await?;

  if response["error"].is_object() {
    return Err(anyhow!("VK API Error: {:#?}", response["error"]));
  }

  if response["response"].is_null() {
    return Err(anyhow!("VK API Error: No response field in API result"));
  }

  Ok(response["response"].clone())
}
