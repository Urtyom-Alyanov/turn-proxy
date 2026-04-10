use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use rand::{RngExt, seq::IndexedRandom as _};
use reqwest::Client;
use serde_json::json;
use tracing::info;

use crate::providers::vk::captcha_solve::redirect_url::{ChallengeMeta, vk_api_request::method_call};

/// Генерирует рандомное железо пользователя
fn random_hardware_info() -> (u8, u8)
{
  let mut rng = rand::rng();
  let cores_options = [4, 8, 12, 16];
  let memory_options = [2, 4, 8];

  let cores = cores_options[rng.random_range(0..cores_options.len())];
  let memory = memory_options[rng.random_range(0..memory_options.len())];

  (cores, memory)
}

/// Cоздаёт окружение для решения задачи, вызывая необходимые методы VK API,
/// чтобы казаться "живым" пользователем
///
/// Возвращает `HashMap` с `domain`, `adFp`, `session_token` и `access_token`, которые
/// ассоциируются с решённой PoW задачей
pub async fn create_captcha_environment(
  client: &Client,
  meta: &ChallengeMeta
) -> Result<HashMap<String, String>>
{
  let base_body = HashMap::from([
    ("session_token".to_owned(), meta.session_token.clone()),
    ("domain".to_owned(), "vk.com".to_owned()),
    ("adFp".to_owned(), "".to_owned()),
    (
      "access_token".to_owned(),
      meta.access_token.clone(),
    ),
  ]);

  tokio::time::sleep(random_sleep()).await;

  let settings = method_call(
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
    "languages": ["ru-RU", "ru", "en-US", "en"],
    "webdriver":false,
    "hardwareConcurrency": cores,
    "deviceMemory": memories,
    "connectionEffectiveType":"4g",
    "notificationsPermission":"denied"
  })
  .to_string();

  let mut done_data = HashMap::from([
    ("device".to_owned(), device_json),
    ("browser_fp".to_owned(), meta.metrics.browser_fingerprint.clone()),
  ]);

  done_data.extend(base_body.clone());

  method_call(client, "captchaNotRobot.componentDone", done_data)
    .await?;
  tokio::time::sleep(random_sleep()).await;

  Ok(base_body)
}

/// Рангдомное количество, нужно для того, чтобы в интернет метриках и курсоре было равное количество элементов
fn random_count() -> usize
{
  let mut rng = rand::rng();
  rng.random_range(5..15)
}

/// Генерирует рандомные координаты курсора для имитации движения мыши при
/// решении PoW задачи
fn cursor(steps: usize) -> String
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

/// Создаёт интернет-метрики настоящева пользувателя для ВК
/// 
/// Возвращает rtt и downlink
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

/// Структура с метриками как бы "реального" пользователя
#[derive(Clone)]
pub struct HumanMetrics {
  // Количество элементов в метриках
  // pub count: usize,

  // Статистика интернета
  pub rtt: String,
  pub downlink: String,

  // Статистика использования
  pub cursor: String,
  pub accelerometer: String,
  pub gyroscope: String,
  pub motion: String,
  pub taps: String,

  // Отпечаток браузера
  pub browser_fingerprint: String
}

impl HumanMetrics {
  pub fn into_hashmap(&self) -> HashMap<String, String> {
    HashMap::from([
      ("browser_fp".to_owned(), self.browser_fingerprint.clone()),
      ("cursor".to_owned(), self.cursor.clone()),
      ("accelerometer".to_owned(), self.accelerometer.clone()),
      ("gyroscope".to_owned(), self.gyroscope.clone()),
      ("motion".to_owned(), self.motion.clone()),
      ("taps".to_owned(), self.taps.clone()),
      ("connectionRtt".to_owned(), self.rtt.clone()),
      ("connectionDownlink".to_owned(), self.downlink.clone()),
    ])
  }
}

/// Создаёт метрики с переданным или сгенерированным количеством 
pub fn create_human_metrics(count: Option<usize>) -> HumanMetrics
{
  let count = count.unwrap_or(random_count());

  let cursor = cursor(count);
  let (rtt, downlink) = internet_metrics(count);

  let browser_fingerprint = format!("{:032x}", rand::random::<u64>());
  
  let empty_array_string = "[]";

  HumanMetrics {
    browser_fingerprint,

    // count,
    rtt,
    downlink,
    cursor,

    // Этого на ПК нет (мы кстати ПуКа)
    accelerometer: empty_array_string.to_owned(),
    gyroscope: empty_array_string.to_owned(),
    motion: empty_array_string.to_owned(),
    taps: empty_array_string.to_owned(),
  }
}

/// Рандомное засыпания, чтобы запросы выполнялись с небольшим интервалом, словно блестяще медленный JS
fn random_sleep() -> Duration
{
  Duration::from_millis(rand::rng().random_range(200..400))
}