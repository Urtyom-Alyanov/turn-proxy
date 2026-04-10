use std::collections::HashMap;

use anyhow::{Result, anyhow};
use reqwest::Client;
use tracing::warn;

use crate::providers::vk::captcha_solve::redirect_url::{
  ChallengeAnswer, ChallengeMeta, DEBUG_INFO,
  reverse_proxy::solve_via_reverse_proxy, vk_api_request::method_call,
};

/// Отправляет на сервера ВКонтукле решение и тихо, без суеты, завершает сессию
/// капчи, если ошибка - запускает реверсивный прокси сервер
/// Ответ - `success_token`
pub async fn submit_captcha(
  client: &Client,
  meta: ChallengeMeta,
  answer: ChallengeAnswer,
  base_body: HashMap<String, String>,
) -> Result<String>
{
  let mut body: HashMap<String, String> =
    HashMap::from([("debug".to_owned(), DEBUG_INFO.to_owned())]);

  body.extend(base_body);
  body.extend(answer.into_hashmap());
  body.extend(meta.metrics.into_hashmap());

  let resp = method_call(client, "captchaNotRobot.check", body).await?;

  if resp["status"].as_str() != Some("OK") {
    warn!("PoW solver has ben rejected! Trying manual solving...");
    let token = solve_via_reverse_proxy(client, &meta.redirect_url).await?;

    if token.is_empty() {
      return Err(anyhow!(
        "PoW solution rejected. Reason: {:#?}",
        resp["status"].as_str().unwrap()
      ));
    }

    return Ok(token);
  }

  let success_token = resp["success_token"]
    .as_str()
    .ok_or_else(|| anyhow!("No success_token in response"))?
    .to_owned();

  Ok(success_token)
}
