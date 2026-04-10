// use anyhow::{Result,Context};
// use regex::Regex;
// use serde::Deserialize;
// use serde_json::Value;

// #[derive(Debug)]
// pub struct CaptchaBootstrap {
//   pub slider_settings: String,
// }

// #[derive(Deserialize, Debug)]
// pub struct WindowInit {
//   pub data: InitData,
// }

// #[derive(Deserialize, Debug)]
// pub struct InitData {
//   pub session_token: String,
//   pub captcha_settings: Vec<Value>,
// }

// pub fn get_captcha_type(html: &str) -> Result<CaptchaBootstrap>
// {
//   let re_init = Regex::new(r#"(?s)window\.init\s*=\s*(\{.*?\})\s*;\s*window\.lang"#)?;
//   let init_json_str = re_init
//       .captures(html)
//       .context("window.init not found")?[1]
//       .to_string();

//   let init_data: WindowInit = serde_json::from_str(&init_json_str)
//       .context("Failed to parse window.init JSON")?;

//   let mut slider_settings = String::new();
//   for item in init_data.data.captcha_settings {
//     if item["type"] == "slider" {
//       if let Some(s) = item["settings"].as_str() {
//           slider_settings = s.to_string();
//       }
//     }
//   }

//   Ok(CaptchaBootstrap {
//     slider_settings
//   })
// }