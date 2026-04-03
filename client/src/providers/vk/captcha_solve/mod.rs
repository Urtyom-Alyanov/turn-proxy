pub mod redirect_uri;
pub mod image_view;
mod reverse_proxy;
pub const PROXY_ADDR: &str = "127.0.0.1:8765";
pub const IMAGE_SERVER_ADDR: &str = "127.0.0.1:8765";

use std::collections::HashMap;
use anyhow::{anyhow, Result};
use serde_json::{Map, Value};
use crate::providers::vk::captcha_solve::image_view::solve_captcha_via_image;
use crate::providers::vk::captcha_solve::redirect_uri::solve_captcha_via_proxy;

/// Выводит пользователю капчу (либо страницу от ВК, либо картинку).
///
/// Возвращает `HashMap` с заполненными `captcha_sid` и `captcha_key` (или `success_token`),
/// а также с `captcha_ts` и `captcha_attempt` при их имении в `err_obj`
pub async fn solve_captcha(
  err_obj: Map<String, Value>,
  attempt: usize,
  max_attempts: usize,
) -> Result<HashMap<String, String>> {
  let code = err_obj.get("error_code").and_then(|v| v.as_i64()).unwrap_or(0);

  if code != 14 {
    return Err(anyhow!("VK API Error: {:#?}", err_obj));
  }

  if attempt >= max_attempts {
    return Err(anyhow!("Captcha failed after {} attempts", max_attempts));
  }

  let sid = err_obj
    .get("captcha_sid")
    .map(|v| v.to_string())
    .ok_or_else(|| anyhow::anyhow!("No captcha_sid provided by VK"))?;

  let mut params = HashMap::new();
  params.insert("captcha_sid".to_owned(), sid);

  let redirect_uri = err_obj.get("redirect_uri").and_then(|v| v.as_str()).unwrap_or("");

  if !redirect_uri.is_empty() {
    let success_token = solve_captcha_via_proxy(redirect_uri).await?.to_string();

    params.insert("success_token".to_owned(), success_token);

    if let Some(ts) = err_obj.get("captcha_ts") {
      params.insert("captcha_ts".to_owned(), ts.to_string());
    }
    if let Some(att) = err_obj.get("captcha_attempt") {
      let val = att.as_i64().unwrap_or(1);
      params.insert("captcha_attempt".to_owned(), val.to_string());
    }
  } else {
    let img = err_obj.get("captcha_img").and_then(|v| v.as_str()).unwrap_or("");

    let key = solve_captcha_via_image(img).await?;
    params.insert("captcha_key".to_owned(), key);
  }

  Ok(params)
}