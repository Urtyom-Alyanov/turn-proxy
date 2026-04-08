use std::{collections::HashMap, time::Duration};

use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose};
use rand::{RngExt, seq::IndexedRandom as _};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use regex::Regex;
use reqwest::{Client, Url};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{task::spawn_blocking};
use tracing::info;

use crate::providers::vk::captcha_solve::reverse_proxy::solve_via_reverse_proxy;

/// Извлекает `session_token` из `redirect_uri`, если он там есть
fn extract_session_token(redirect_uri: &str) -> Option<String>
{
  let parsed_url = Url::parse(redirect_uri).ok()?;

  parsed_url
    .query_pairs()
    .find(|(key, _)| key == "session_token")
    .map(|(_, value)| value.into_owned())
}

/// Генерирует рандомные значения скорости соединения для имитации реального
/// пользователя при решении PoW задачи
// fn generate_random_downlink() -> String
// {
//   let mut rng = rand::rng();

//   let base_speed: f32 = rng.random_range(5.0..15.0);

//   let mut samples = Vec::new();
//   for _ in 0..16 {
//     let noise = rng.random_range(-1.2..1.3);
//     let val = (base_speed + noise).clamp(0.5, 15.0);

//     samples.push(format!("{:.1}", val));
//   }

//   format!("[{}]", samples.join(","))
// }

/// Генерирует рандомные координаты курсора для имитации движения мыши при
/// решении PoW задачи
fn generate_random_cursor(steps: usize) -> String
{
  let mut rng = rand::rng();
  let mut points = Vec::new();

  // Начальная точка (где-то в области контента)
  let mut curr_x = rng.random_range(100..900) as i32;
  let mut curr_y = rng.random_range(100..700) as i32;

  // Направление движения (куда "ползет" мышь)
  let mut dx = rng.random_range(-2..=2);
  let mut dy = rng.random_range(-2..=2);

  for i in 0..steps {
    // Каждые несколько шагов немного меняем вектор направления (плавный
    // поворот)
    if i % 5 == 0 {
      dx += rng.random_range(-1..=1);
      dy += rng.random_range(-1..=1);
    }

    // Добавляем микро-дрожание (jitter)
    let jitter_x = rng.random_range(-1..=1);
    let jitter_y = rng.random_range(-1..=1);

    curr_x += dx + jitter_x;
    curr_y += dy + jitter_y;

    points.push(json!({
        "x": curr_x,
        "y": curr_y
    }));

    // Иногда мышь замирает на месте (имитируем микро-паузы пользователя)
    if rng.random_bool(0.1) {
      points.push(json!({
          "x": curr_x,
          "y": curr_y
      }));
    }
  }

  serde_json::to_string(&points).unwrap_or_else(|_| "[]".to_string())
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

  info!(
    "Starting PoW challenge solve, session_token: {}",
    session_token
  );

  let (pow_input, difficulty) =
    fetch_pow_challenge(client, redirect_url).await?;

  info!("Getted input and difficulty: {}, {}", pow_input, difficulty);

  let hash_handle = spawn_blocking(move || solve_pow(&pow_input, difficulty));

  let browser_fp = format!("{:032x}", rand::random::<u64>());

  let (hash_handler, base_body_handler) = tokio::join!(
    hash_handle,
    create_pow_environment(client, session_token, access_token, &browser_fp)
  );

  let hash = hash_handler??;
  let base_body = base_body_handler?;

  info!("Getted hash (solve): {}", hash);

  let (success_token, is_human) =
    submit_pow_solution(client, &hash, base_body.clone(), &browser_fp, redirect_url).await?;

  info!("Getted success token: {}", success_token);

  if is_human {
    let _ =
    vk_api_request_internal(client, "captchaNotRobot.endSession", base_body)
      .await;
    info!("Gracefully ending PoW session...");
  }
  
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
  let full_bytes = difficulty / 2;
  let has_half_byte = difficulty % 2 != 0;
  (0..u64::MAX)
    .into_par_iter()
    .find_first(|&nonce| {
      let mut hasher = Sha256::new();
      let data = format!("{}{}", pow_input, nonce);

      hasher.update(data.as_bytes());
      let hash = hasher.finalize();

      for i in 0..full_bytes {
        if hash[i] != 0 {
          return false;
        }
      }

      if has_half_byte {
        if (hash[full_bytes] & 0xF0) != 0 {
          return false;
        }
      }

      true
    })
    .map(|nonce| {
      let data = format!("{}{}", pow_input, nonce);
      let mut hasher = Sha256::new();
      hasher.update(data.as_bytes());
      hex::encode(hasher.finalize())
    })
    .ok_or_else(|| anyhow!("Failed to solve PoW challenge"))
  // Err(anyhow!("Failed to solve PoW challenge"))
}

fn internet_metrics(count: usize) -> (String, String)
{
  let mut rng = rand::rng();

  let rtt_val = *[50, 100, 200].choose(&mut rng).unwrap();
  let rtt = format!(
    "[{}]",
    (0..count)
      .map(|_| rtt_val.to_string())
      .collect::<Vec<_>>()
      .join(",")
  );

  let dl_val = *[10.0, 15.0, 9.5].choose(&mut rng).unwrap();
  let downlink = format!(
    "[{}]",
    (0..count)
      .map(|_| format!("{:.1}", dl_val))
      .collect::<Vec<_>>()
      .join(",")
  );

  (rtt, downlink)
}

fn random_count() -> usize
{
  let mut rng = rand::rng();
  rng.random_range(5..15)
}

/// Отправляет решение PoW задачи на сервер и получает токен успеха
/// 
/// Первый объект в результате - токен, второй - ручной ли ввод
async fn submit_pow_solution(
  client: &Client,
  hash: &str,
  base_body: HashMap<String, String>,
  browser_fp: &str,
  redirect_url: &str,
) -> Result<(String, bool)>
{
  let count = random_count();

  let cursor = generate_random_cursor(count);
  let answer = general_purpose::STANDARD.encode("{}");

  let empty_array_string = "[]";

  let (rtt, connection_downlink) = internet_metrics(count);

  info!("Generated cursor: {}", cursor);
  info!("Generated connection downlink: {}", connection_downlink);

  let mut body = HashMap::from([
    ("hash".to_owned(), hash.to_owned()),
    ("answer".to_owned(), answer),
    ("browser_fp".to_owned(), browser_fp.to_owned()),
    ("cursor".to_owned(), cursor),
    ("accelerometer".to_owned(), empty_array_string.to_owned()),
    ("gyroscope".to_owned(), empty_array_string.to_owned()),
    ("motion".to_owned(), empty_array_string.to_owned()),
    ("taps".to_owned(), empty_array_string.to_owned()),
    ("connectionRtt".to_owned(), rtt),
    ("connectionDownlink".to_owned(), connection_downlink),
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
    let token = solve_via_reverse_proxy(client, redirect_url).await?;

    if token.is_empty() {
      return Err(anyhow!(
        "PoW solution rejected. Reason: {:#?}",
        resp["status"].as_str().unwrap()
      ));
    }
    
    return Ok((token, true));
  }

  let success_token = resp["success_token"]
    .as_str()
    .ok_or_else(|| anyhow!("No success_token in response"))?
    .to_owned();

  Ok((success_token, false))
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

  tokio::time::sleep(random_sleep()).await;

  vk_api_request_internal(
    client,
    "captchaNotRobot.settings",
    base_body.clone(),
  )
  .await?;
  tokio::time::sleep(random_sleep()).await;

  let (cores, memories) = random_hardware_info();

  let device_json = json!({
    "screenWidth": 1920,
    "screenHeight": 1080,
    "screenAvailWidth":1920,
    "screenAvailHeight":1032,
    "innerWidth":1920,
    "innerHeight":945,
    "devicePixelRatio": 1,
    "language": "ru-RU",
    "languages":["ru-RU", "ru", "en-US", "en"],
    "webdriver":false,
    "hardwareConcurrency": cores,
    "deviceMemory": memories,
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
  tokio::time::sleep(random_sleep()).await;

  Ok(base_body)
}

fn random_hardware_info() -> (u8, u8)
{
  let mut rng = rand::rng();
  let cores_options = [4, 8, 12, 16];
  let memory_options = [2, 4, 8];

  let cores = cores_options[rng.random_range(0..cores_options.len())];
  let memory = memory_options[rng.random_range(0..memory_options.len())];

  (cores, memory)
}

fn random_sleep() -> Duration
{
  Duration::from_millis(rand::rng().random_range(200..400))
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
