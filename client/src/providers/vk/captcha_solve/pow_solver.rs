use std::{collections::HashMap, time::Duration};

use anyhow::{Result, anyhow};
use base64::{Engine, engine::general_purpose, prelude::BASE64_STANDARD};
use image::{DynamicImage, GenericImageView as _, Rgba, load_from_memory};
use rand::{RngExt, seq::IndexedRandom as _};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use regex::Regex;
use reqwest::{Client, Url};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::task::spawn_blocking;
use tracing::{debug, info, warn};

use crate::providers::vk::captcha_solve::reverse_proxy::solve_via_reverse_proxy;

const DEBUG_INFO: &str = "1d3e9babfd3a74f4588bf90cf5c30d3e8e89a0e2a4544da8de8bbf4d78a32f5c";

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

/// Получает URL картинки с сервера ВК (ссылка base64)
async fn fetch_picture(
  client: &Client,
  picture_id: &str,
  session_token: &str,
  access_token: Option<&str>
) -> Result<DynamicImage>
{
  let method_body = HashMap::from([
    ("captcha_settings".to_owned(), picture_id.to_owned()),
    ("session_token".to_owned(), session_token.to_owned()),
    ("domain".to_owned(), "vk.com".to_owned()),
    ("adFp".to_owned(), "".to_owned()),
    (
      "access_token".to_owned(),
      access_token.unwrap_or("").to_owned(),
    ),
  ]);

  let body = vk_api_request_internal(client, "captchaNotRobot.getContent", method_body).await?;
  let content = body["image"].as_str().ok_or(anyhow!("Can't resolve image content for captcha"))?;
  let extension = body["extension"].as_str().ok_or(anyhow!("Can't resolve image extension for captcha"))?;
  
  info!("Fetched image captcha challenge");
  debug!("Base64 URL: data:image/{};base64,{}", extension, content);

  let image_bytes = BASE64_STANDARD
    .decode(content.trim())
    .map_err(|e| anyhow!("Decode Base64 error: {}", e))?;

  let img = load_from_memory(&image_bytes)
    .map_err(|e| anyhow!("Loading image buffer into image type: {}", e))?;

  Ok(img)
}

#[derive(Debug)]
pub struct SliderCandidate {
    pub index: usize,
    pub active_steps: Vec<i32>,
    pub score: u64,
}

/// Вычисляет разницу между двумя пикселями (R+G+B)
fn pixel_diff(p1: Rgba<u8>, p2: Rgba<u8>) -> u64 {
    let r_diff = (p1[0] as i32 - p2[0] as i32).abs() as u64;
    let g_diff = (p1[1] as i32 - p2[1] as i32).abs() as u64;
    let b_diff = (p1[2] as i32 - p2[2] as i32).abs() as u64;
    r_diff + g_diff + b_diff
}

/// Считает "плохость" границ при текущем маппинге тайлов
fn score_mapping(img: &image::DynamicImage, grid_size: usize, mapping: &[usize]) -> u64 {
  let (width, height) = img.dimensions();
  let tile_w = width / grid_size as u32;
  let tile_h = height / grid_size as u32;
  let mut total_score = 0u64;

  // Функция получения координат пикселя с учетом перемешанных тайлов
  let get_pixel = |target_tile_idx: usize, local_x: u32, local_y: u32| {
    let source_tile_idx = mapping[target_tile_idx];
    let src_col = (source_tile_idx % grid_size) as u32;
    let src_row = (source_tile_idx / grid_size) as u32;
    img.get_pixel(src_col * tile_w + local_x, src_row * tile_h + local_y)
  };

  for row in 0..grid_size {
    for col in 0..grid_size {
      let current_tile = row * grid_size + col;

      // Проверяем правую границу тайла (с левой границей соседа справа)
      if col < grid_size - 1 {
        let next_tile = current_tile + 1;
        for y in 0..tile_h {
          let p_left = get_pixel(current_tile, tile_w - 1, y);
          let p_right = get_pixel(next_tile, 0, y);
          total_score += pixel_diff(p_left, p_right);
        }
      }

      // Проверяем нижнюю границу тайла (с верхней границей соседа снизу)
      if row < grid_size - 1 {
        let bottom_tile = current_tile + grid_size;
        for x in 0..tile_w {
          let p_top = get_pixel(current_tile, x, tile_h - 1);
          let p_bottom = get_pixel(bottom_tile, x, 0);
          total_score += pixel_diff(p_top, p_bottom);
        }
      }
    }
  }
  total_score
}

/// Главная функция ранжирования
pub fn rank_candidates(
    img_bytes: &[u8],
    grid_size: usize,
    swaps: &[i32],
) -> anyhow::Result<Vec<SliderCandidate>> {
    let img = image::load_from_memory(img_bytes)?;
    let tile_count = grid_size * grid_size;
    let candidate_count = swaps.len() / 2;

    let mut candidates = Vec::new();

    let mut current_mapping: Vec<usize> = (0..tile_count).collect();

    for i in 1..=candidate_count {
        let idx1 = swaps[(i - 1) * 2] as usize;
        let idx2 = swaps[(i - 1) * 2 + 1] as usize;
        current_mapping.swap(idx1, idx2);

        let score = score_mapping(&img, grid_size, &current_mapping);
        
        candidates.push(SliderCandidate {
            index: i,
            active_steps: swaps[0..(i * 2)].to_vec(),
            score,
        });
    }

    candidates.sort_by_key(|c| c.score);
    Ok(candidates)
}

/// Решение задачи со слайдером путём поиска стыков
async fn solve_picture(
  client: &Client,
  picture_id: &str,
  session_token: &str,
  access_token: Option<&str>
) -> Result<String>
{
  let image_buffer = fetch_picture(client, picture_id, session_token, access_token).await?;



  Ok(String::new())
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

  let (pow_input, difficulty, captcha_image_id) =
    fetch_pow_challenge(client, redirect_url).await?;

  info!("Getted input and difficulty: {}, {}", pow_input, difficulty);

  let hash_handle = spawn_blocking(move || solve_pow(&pow_input, difficulty));

  let browser_fp = format!("{:032x}", rand::random::<u64>());
  
  let _: Option<String> = match captcha_image_id {
    Some(image_id) => Some(solve_picture(client, &image_id, session_token, access_token).await?),
    None => None
  };

  let (hash_handler, base_body_handler,) = tokio::join!(
    hash_handle,
    create_pow_environment(client, session_token, access_token, &browser_fp)
  );

  let hash = hash_handler??;
  let base_body = base_body_handler?;

  info!("Getted hash (solve): {}", hash);

  let (success_token, is_human) = submit_pow_solution(
    client,
    &hash,
    base_body.clone(),
    &browser_fp,
    redirect_url,
  )
  .await?;

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
/// 
/// Возвращает: input, difficulty, slider_image_id
async fn fetch_pow_challenge(
  client: &Client,
  redirect_url: &str,
) -> Result<(String, usize, Option<String>)>
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

  Ok((pow_input, difficulty, slider_image_id))
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
      DEBUG_INFO.to_owned(),
    ),
  ]);

  body.extend(base_body);

  let resp =
    vk_api_request_internal(client, "captchaNotRobot.check", body).await?;

  if resp["status"].as_str() != Some("OK") {
    warn!("PoW solver has ben rejected! Trying manual solving...");
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

  let settings = vk_api_request_internal(
    client,
    "captchaNotRobot.settings",
    base_body.clone(),
  )
  .await?;
  info!("Current settings: {}", settings);

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
