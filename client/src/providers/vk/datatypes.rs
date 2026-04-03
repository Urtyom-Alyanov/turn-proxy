use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestParam
{
  pub key: String,
  pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptchaError
{
  pub error_code: i32,
  pub error_message: String,

  // Сама капча (redirect_uri и тд)
  pub redirect_uri: String,
  pub captcha_attempt: u32,
  pub captcha_img: String,
  pub captcha_ratio: f32,
  pub captcha_sid: String,
  pub captcha_track: String,
  pub captcha_ts: f64,

  // UI/UX поля для клиента
  pub is_refresh_enabled: bool,
  pub is_sound_captcha_available: bool,
  pub uiux_changes: bool,

  pub remixstlid: u64,

  pub request_params: Vec<RequestParam>,
}
