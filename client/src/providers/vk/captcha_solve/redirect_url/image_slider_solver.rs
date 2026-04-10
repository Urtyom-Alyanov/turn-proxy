use std::collections::HashMap;

use base64::{Engine, prelude::*};
use image::{DynamicImage, GenericImageView as _, Rgba, load_from_memory};
use rayon::prelude::*;
use reqwest::Client;
use anyhow::{Result,anyhow};
use serde_json::json;
use tracing::{debug, info};

use crate::providers::vk::captcha_solve::redirect_url::{ChallengeMeta, vk_api_request::method_call};

struct SliderContent {
  image: DynamicImage,
  grid_size: i32,
  swaps: Vec<i32>
}

/// Рассчитывает разницу между двумя пикселями (RGB)
fn pixel_diff(p1: Rgba<u8>, p2: Rgba<u8>) -> i64 {
  (p1[0] as i64 - p2[0] as i64).abs() +
  (p1[1] as i64 - p2[1] as i64).abs() +
  (p1[2] as i64 - p2[2] as i64).abs()
}

/// Оценивает "стыковку" плиток для конкретной перестановки
fn score_mapping(img: &DynamicImage, grid_size: i32, mapping: &[usize]) -> i64 {
  let (width, height) = img.dimensions();
  let tile_w = width / grid_size as u32;
  let tile_h = height / grid_size as u32;
  let mut total_score = 0i64;

  for row in 0..grid_size {
    for col in 0..grid_size {
      let current_idx = (row * grid_size + col) as usize;
      let src_idx = mapping[current_idx];

      // Координаты текущей плитки в исходном изображении
      let src_x = (src_idx as i32 % grid_size) as u32 * tile_w;
      let src_y = (src_idx as i32 / grid_size) as u32 * tile_h;

      // Сравнение с правой соседней плиткой
      if col < grid_size - 1 {
        let right_src_idx = mapping[current_idx + 1];
        let r_src_x = (right_src_idx as i32 % grid_size) as u32 * tile_w;
        let r_src_y = (right_src_idx as i32 / grid_size) as u32 * tile_h;

        for y_off in 0..tile_h {
          let p_left = img.get_pixel(src_x + tile_w - 1, src_y + y_off);
          let p_right = img.get_pixel(r_src_x, r_src_y + y_off);
          total_score += pixel_diff(p_left, p_right);
        }
      }

      // Сравнение с нижней соседней плиткой
      if row < grid_size - 1 {
        let bottom_src_idx = mapping[current_idx + grid_size as usize];
        let b_src_x = (bottom_src_idx as i32 % grid_size) as u32 * tile_w;
        let b_src_y = (bottom_src_idx as i32 / grid_size) as u32 * tile_h;

        for x_off in 0..tile_w {
          let p_top = img.get_pixel(src_x + x_off, src_y + tile_h - 1);
          let p_bottom = img.get_pixel(b_src_x + x_off, b_src_y);
          total_score += pixel_diff(p_top, p_bottom);
        }
      }
    }
  }
  total_score
}

/// Получает URL картинки с сервера ВК (ссылка base64)
async fn fetch_picture(
  client: &Client,
  captcha_settings: &str,
  meta: ChallengeMeta
) -> Result<SliderContent>
{
  let method_body = HashMap::from([
    ("captcha_settings".to_owned(), captcha_settings.to_owned()),
    ("session_token".to_owned(), meta.session_token.clone()),
    ("domain".to_owned(), "vk.com".to_owned()),
    ("adFp".to_owned(), "".to_owned()),
    (
      "access_token".to_owned(),
      meta.access_token.clone(),
    ),
  ]);

  let body = method_call(client, "captchaNotRobot.getContent", method_body).await?;
  let content = body["image"].as_str().ok_or(anyhow!("Can't resolve image content for captcha"))?;
  let extension = body["extension"].as_str().ok_or(anyhow!("Can't resolve image extension for captcha"))?;
  
  info!("Fetched image captcha challenge");
  debug!("Base64 URL: data:image/{};base64,{}", extension, content);

  let image_bytes = BASE64_STANDARD
    .decode(content.trim())
    .map_err(|e| anyhow!("Decode Base64 error: {}", e))?;

  let image = load_from_memory(&image_bytes)
    .map_err(|e| anyhow!("Loading image buffer into image type: {}", e))?;

  let raw_steps = body["steps"]
    .as_array()
    .ok_or(anyhow!("No steps in slider content"))?;

  let steps: Vec<i32> = raw_steps
    .iter()
    .map(|v| v.as_i64().unwrap_or(0) as i32)
    .collect();
  
  if steps.len() < 3 {
    return Err(anyhow!("Slider steps payload too short"));
  }

  let grid_size = steps[0];
  let swaps = steps[1..].to_vec();
  
  // let mut attempts = 4;
  // if swaps.len() % 2 != 0 {
  //     attempts = swaps.pop().unwrap_or(4);
  // }

  Ok(
    SliderContent { image, grid_size, swaps }
  )
}

/// Решение задачи со слайдером путём поиска стыков
pub async fn solve_picture(
  client: &Client,
  picture_id: &str,
  meta: ChallengeMeta
) -> Result<String>
{
  let content = fetch_picture(client, picture_id, meta).await?;

  let candidate_count = content.swaps.len() / 2;

  // Начальное состояния без перемещений
  let tile_count = (content.grid_size * content.grid_size) as usize;

  let best_result = (1..=candidate_count)
    .into_par_iter()
    .map(|i| {
        // Для каждого кандидата воссоздаем его состояние маппинга
        let mut mapping: Vec<usize> = (0..tile_count).collect();
        for step in 0..i {
          let a = content.swaps[step * 2] as usize;
          let b = content.swaps[step * 2 + 1] as usize;
          mapping.swap(a, b);
        }
        
        let score = score_mapping(&content.image, content.grid_size, &mapping);
        (i, score)
    })
    .min_by_key(|&(_, score)| score);

  if let Some((best_index, score)) = best_result {
    info!("Parallel solver found best index: {} (score: {})", best_index, score);
    let best_swaps = content.swaps[0..(best_index * 2)].to_vec();
    let answer = BASE64_STANDARD.encode(json!({ "value": best_swaps }).to_string());
    Ok(answer)
  } else {
    Err(anyhow!("No candidates found"))
  }
}