use std::{collections::HashMap, time::Duration};

use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use regex::Regex;
use reqwest::{Client, Url};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;

/// Извлекает `session_token` из `redirect_uri`, если он там есть
fn extract_session_token(redirect_uri: &str) -> Option<String>
{
  let parsed_url = Url::parse(redirect_uri).ok()?;

  parsed_url
    .query_pairs()
    .find(|(key, _)| key == "session_token")
    .map(|(_, value)| value.into_owned())
}

/// Решает PoW задачу, предоставленную ВК, и получает `success_token`, который
/// можно использовать для обхода капчи.
pub async fn solve_pow_challenge(
  client: &Client,
  redirect_url: &str,
  session_token: Option<&str>,
  access_token: Option<&str>,
) -> Result<String>
{
  let session_token_from_url =
    extract_session_token(redirect_url).unwrap_or("".to_owned());

  let session_token = session_token.unwrap_or(session_token_from_url.as_str());

  let (pow_input, difficulty) =
    fetch_pow_challenge(client, redirect_url).await?;

  let hash_handle = spawn_blocking(move || solve_pow(&pow_input, difficulty));

  let browser_fp = format!("{:032x}", rand::random::<u64>());

  let (hash_handler, base_body_handler) = tokio::join!(
    hash_handle,
    create_pow_environment(client, session_token, access_token, &browser_fp)
  );

  let hash = hash_handler??;
  let base_body = base_body_handler?;

  let success_token =
    submit_pow_solution(client, &hash, base_body, &browser_fp).await?;

  Ok(success_token)
}

/// Извлекает из HTML страницы данные для PoW задачи
async fn fetch_pow_challenge(
  client: &Client,
  redirect_url: &str,
) -> Result<(String, usize)>
{
  let html = client.get(redirect_url).send().await?.text().await?;

  let re_input = Regex::new(r#"const\s+powInput\s*=\s*"([^"]+)""#)?;
  let pow_input = re_input
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

  Ok((pow_input, difficulty))
}

/// Решает PoW задачу, перебирая nonce до тех пор, пока не будет найден хэш,
/// начинающийся с нужного количества нулей
fn solve_pow(pow_input: &str, difficulty: usize) -> Result<String>
{
  let target = "0".repeat(difficulty);
  // for nonce in 1..10_000_000 {
  //   let data = format!("{}{}", pow_input, nonce);
  //   let mut hasher = Sha256::new();
  //   hasher.update(data.as_bytes());
  //   let hex_hash = hex::encode(hasher.finalize());

  //   if hex_hash.starts_with(&target) {
  //     return Ok(hex_hash);
  //   }
  // }
  (0..10_000_000)
    .into_par_iter()
    .find_map_any(|nonce| {
      let data = format!("{}{}", pow_input, nonce);
      let mut hasher = Sha256::new();
      hasher.update(data.as_bytes());
      let hex_hash = hex::encode(hasher.finalize());

      if hex_hash.starts_with(&target) {
        Some(hex_hash)
      } else {
        None
      }
    })
    .ok_or_else(|| anyhow!("Failed to solve PoW challenge"))
  // Err(anyhow!("Failed to solve PoW challenge"))
}

/// Отправляет решение PoW задачи на сервер и получает токен успеха
async fn submit_pow_solution(
  client: &Client,
  hash: &str,
  base_body: HashMap<String, String>,
  browser_fp: &str,
) -> Result<String>
{
  let cursor = r#"[{"x":950,"y":500},{"x":945,"y":510},{"x":940,"y":520},{"x":938,"y":525},{"x":938,"y":525}]"#;
  let answer = general_purpose::STANDARD.encode("{}");

  let empty_array_string = "[]";
  let connection_downlink =
    "[9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5,9.5]";

  let mut body = HashMap::from([
    ("hash".to_owned(), hash.to_owned()),
    ("answer".to_owned(), answer),
    ("browser_fp".to_owned(), browser_fp.to_owned()),
    ("cursor".to_owned(), cursor.to_owned()),
    ("accelerometer".to_owned(), empty_array_string.to_owned()),
    ("gyroscope".to_owned(), empty_array_string.to_owned()),
    ("motion".to_owned(), empty_array_string.to_owned()),
    ("taps".to_owned(), empty_array_string.to_owned()),
    ("connectionRtt".to_owned(), "".to_owned()),
    (
      "connectionDownlink".to_owned(),
      connection_downlink.to_owned(),
    ),
    (
      "debug_info".to_owned(),
      "d44f534ce8deb56ba20be52e05c433309b49ee4d2a70602deeb17a1954257785"
        .to_owned(),
    ),
  ]);

  body.extend(base_body);

  let resp =
    vk_api_request_internal(client, "captchaNotRobot.check", body).await?;

  if resp["status"].as_str() != Some("OK") {
    return Err(anyhow!("PoW solution rejected: {:#?}", resp));
  }

  let success_token = resp["success_token"]
    .as_str()
    .ok_or_else(|| anyhow!("No success_token in response"))?
    .to_owned();

  Ok(success_token)
}

/// Cоздаёт окружение для решения PoW задачи, вызывая необходимые методы VK API,
/// чтобы казаться "живым" пользователем
///
/// Возвращает `HashMap` с `session_token` и `access_token`, которые
/// ассоциируются с решённой PoW задачей
async fn create_pow_environment(
  client: &Client,
  session_token: &str,
  access_token: Option<&str>,
  browser_fp: &str,
) -> Result<HashMap<String, String>>
{
  let base_body = HashMap::from([
    ("session_token".to_owned(), session_token.to_owned()),
    ("domain".to_owned(), "vk.com".to_owned()),
    ("adFp".to_owned(), "".to_owned()),
    (
      "access_token".to_owned(),
      access_token.unwrap_or("").to_owned(),
    ),
  ]);

  vk_api_request_internal(
    client,
    "captchaNotRobot.settings",
    base_body.clone(),
  )
  .await?;
  tokio::time::sleep(Duration::from_millis(200)).await;

  let device_json = json!({
    "screenWidth": 1920,
    "screenHeight": 1080,
    "screenAvailWidth":1920,
    "screenAvailHeight":1032,
    "innerWidth":1920,
    "innerHeight":945,
    "devicePixelRatio": 1,
    "language": "en-US",
    "languages":["en-US"],
    "webdriver":false,
    "hardwareConcurrency":16,
    "deviceMemory":8,
    "connectionEffectiveType":"4g",
    "notificationsPermission":"denied"
  })
  .to_string();

  let mut done_data = HashMap::from([
    ("device".to_owned(), device_json),
    ("browser_fp".to_owned(), browser_fp.to_owned()),
  ]);

  done_data.extend(base_body.clone());

  vk_api_request_internal(client, "captchaNotRobot.componentDone", done_data)
    .await?;
  tokio::time::sleep(Duration::from_millis(200)).await;

  Ok(base_body)
}

/// Внутренняя функция для вызовов VK API
async fn vk_api_request_internal(
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
