use std::collections::HashMap;

use anyhow::Result;
use base64::{Engine, engine::general_purpose};
use reqwest::{Client, Url};

use crate::providers::vk::captcha_solve::redirect_url::{
  fetch::fetch_challenge,
  human_emulating::{HumanMetrics, create_captcha_environment},
  image_slider_solver::solve_picture,
  proof_of_work::{PowChallenge, solve_pow_async},
  submit::submit_captcha,
  vk_api_request::method_call,
};

mod fetch; // Сама задача
mod human_emulating; // Создаёт окружение "живого" пользователя для ВК
mod image_slider_solver; /* Решает задачу поставленную пользователю путём
                           * сопоставления стыков */
mod proof_of_work; // Решение PoW задачи от ВК
mod reverse_proxy; // Модуль для решения капчи вручную
mod submit; // Отправка решения
mod vk_api_request; // Модуль, чтобы делать запросы (там не обрабатывается капча)

pub(super) const DEBUG_INFO: &str =
  "1d3e9babfd3a74f4588bf90cf5c30d3e8e89a0e2a4544da8de8bbf4d78a32f5c";

/// Метаданные для задачи
#[derive(Clone)]
pub(super) struct ChallengeMeta
{
  session_token: String,
  access_token: String,
  metrics: HumanMetrics,
  redirect_url: String,
}

/// Объект с задачей для капчи
#[derive(Clone)]
pub(super) struct Challenge
{
  pub meta: ChallengeMeta,
  pub proof_of_work: PowChallenge,
  pub slider_settings: Option<String>,
}

/// Решение задачи
#[derive(Clone)]
pub(super) struct ChallengeAnswer
{
  pub answer: String,
  pub hash: String,
}

impl ChallengeAnswer
{
  pub fn into_hashmap(&self) -> HashMap<String, String>
  {
    HashMap::from([
      ("answer".to_owned(), self.answer.clone()),
      ("hash".to_owned(), self.hash.clone()),
    ])
  }
}

/// Извлечение `session_token` из `redirect_uri`
pub(super) fn extract_session_token(redirect_uri: &str) -> Option<String>
{
  let parsed_url = Url::parse(redirect_uri).ok()?;

  parsed_url
    .query_pairs()
    .find(|(key, _)| key == "session_token")
    .map(|(_, value)| value.into_owned())
}

async fn human_answer(
  client: &Client,
  slider_settings: Option<String>,
  meta: ChallengeMeta,
) -> Result<String>
{
  let answer = match slider_settings {
    None => general_purpose::STANDARD.encode("{}"),
    Some(settings) => solve_picture(client, &settings, meta).await?,
  };

  Ok(answer)
}

/// Решает все задачи
async fn get_captcha_answer(
  client: &Client,
  challenge: Challenge,
) -> Result<ChallengeAnswer>
{
  let (answer, hash) = tokio::join!(
    human_answer(client, challenge.slider_settings, challenge.meta),
    solve_pow_async(challenge.proof_of_work)
  );

  Ok(ChallengeAnswer {
    answer: answer?,
    hash: hash??,
  })
}

pub async fn solve_smart_captcha(
  client: &Client,
  redirect_url: &str,
  access_token: Option<&str>,
) -> Result<String>
{
  let challenge = fetch_challenge(client, redirect_url, access_token).await?;

  let (answer_handler, base_body_handler) = tokio::join!(
    get_captcha_answer(client, challenge.clone()),
    create_captcha_environment(client, &challenge.meta)
  );

  let answer = answer_handler?;
  let base_body = base_body_handler?;

  let success_token =
    submit_captcha(client, challenge.meta, answer, base_body.clone()).await?;

  let _ = method_call(client, "captchaNotRobot.endSession", base_body).await;

  Ok(success_token)
}
