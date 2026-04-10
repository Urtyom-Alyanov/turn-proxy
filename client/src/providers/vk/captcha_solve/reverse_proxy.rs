use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use axum::{
  Json, Router,
  body::Body,
  extract::{Query, State},
  http::{HeaderMap, Method},
  response::{Html, IntoResponse, Response},
  routing::{any, get, post},
};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::sync::{
  Mutex,
  oneshot::{Receiver, Sender, channel},
};
use tracing::{error, info};

use crate::providers::vk::captcha_solve::PROXY_ADDR;

const INJECT_JS: &str = include_str!("./inject.js");

struct ProxyContext
{
  redirect_url: reqwest::Url,
  http_client: Client,
  token: Mutex<Option<Sender<String>>>,
}

/// Запускает локальный HTTP сервер, который отображает страницу для ввода капчи
/// и слушает результат.
async fn run_proxy_server(
  redirect_url: &str,
  client: &Client,
  token_channel: Sender<String>,
  shutdown_channel: Receiver<()>,
) -> Result<()>
{
  let ctx = Arc::new(ProxyContext {
    redirect_url: reqwest::Url::parse(redirect_url).unwrap(),
    http_client: client.clone(),
    token: Mutex::new(Some(token_channel)),
  });

  let router = Router::new()
    .route("/", get(captcha_user_input_handler))
    .route("/request", any(captcha_request_handler))
    .route("/submit", post(captcha_submit_handler))
    .with_state(ctx);

  let listener = tokio::net::TcpListener::bind(PROXY_ADDR).await?;
  info!("Opened socket at {} for reverse proxy", PROXY_ADDR);

  println!("http://{} - Manual captcha solving", PROXY_ADDR);

  axum::serve(listener, router)
    .with_graceful_shutdown(async move {
      let _ = shutdown_channel.await;
    })
    .await?;

  Ok(())
}

#[derive(Deserialize)]
struct CaptchaSubmitHandlerPayload
{
  pub token: String,
}

/// Здесь, получается `success_token` от решения капчи, который мы потом
/// записываем в `Mutex`
async fn captcha_submit_handler(
  State(ctx): State<Arc<ProxyContext>>,
  Json(payload): Json<CaptchaSubmitHandlerPayload>,
) -> StatusCode
{
  let mut channel_lock = ctx.token.lock().await;

  info!("Submitting captcha with `success_token`");

  if let Some(channel) = channel_lock.take() {
    let _ = channel.send(payload.token);
    StatusCode::OK
  } else {
    StatusCode::GONE
  }
}

/// Обрабатывает запросы от VK API, перенаправляя их на страницу ввода капчи и
/// ожидая решения от пользователя
///
/// Для ВК показывается USER_AGENT нашего прокси, который имитирует браузер,
/// через который идёт подключение, а не реального клиента, что позволяет обойти
/// некоторые проверки и успешно решать капчи.
async fn captcha_request_handler(
  State(ctx): State<Arc<ProxyContext>>,
  method: Method,
  headers: HeaderMap,
  Query(params): Query<HashMap<String, String>>,
  body: Body,
) -> impl IntoResponse
{
  let target = match params.get("target") {
    Some(t) => t,
    None => return StatusCode::BAD_REQUEST.into_response(),
  };

  let mut request_builder = ctx
    .http_client
    .request(method, target)
    .body(reqwest::Body::wrap_stream(body.into_data_stream()));

  for (name, value) in headers.iter() {
    let name_s = name.as_str().to_lowercase();

    if name_s == "user-agent"
      || name_s.starts_with("sec-ch-ua")
      || name_s == "content-length"
      || name_s == "connection"
      || name_s == "accept-encoding"
    {
      continue;
    }

    if name_s == "host" || name_s == "referer" {
      continue;
    }

    request_builder = request_builder.header(name, value);
  }

  let resp = match request_builder.send().await {
    Ok(r) => r,
    Err(e) => {
      error!("On request error: {}", e);
      return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
  };

  let status = resp.status();
  let mut response_builder = Response::builder().status(status.as_u16());

  if let Some(headers_mut) = response_builder.headers_mut() {
    for (name, value) in resp.headers().iter() {
      headers_mut.insert(name, value.clone());
    }
  }

  let stream = resp.bytes_stream();
  let body = Body::from_stream(stream);

  response_builder.body(body).unwrap()
}

/// Страница для ввода капчи пользователем (инжектится кастомный JS, который всё
/// перенаправляет в `/request`, если в поле есть `success_token` то шлёт его в
/// `/submit`)
async fn captcha_user_input_handler(
  State(ctx): State<Arc<ProxyContext>>,
) -> Result<Html<String>, StatusCode>
{
  info!("Fetching {}...", ctx.redirect_url);

  let response = ctx
    .http_client
    .get(ctx.redirect_url.clone())
    .send()
    .await
    .map_err(|_| StatusCode::BAD_GATEWAY)?;

  let mut html = response
    .text()
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

  info!("Preparing html...");

  html = html.replace("href=\"/\"", "href=\"https://id.vk.ru/\"");
  html = html.replace("src=\"/\"", "src=\"https://id.vk.ru/\"");

  info!("Injecting JS...");
  html =
    html.replace("<head>", &format!("<head><script>{}</script>", INJECT_JS));

  Ok(Html(html))
}

/// Основная функция для решения капчи через обратный прокси. Запускает
/// локальный сервер, который отображает страницу для ввода капчи и слушает
/// результат. После получения решения капчи, извлекает `session_token` и
/// возвращает его. Этот токен можно использовать для повторной отправки запроса
/// к VK API уже с решённой капчей.
pub async fn solve_via_reverse_proxy(
  client: &Client,
  redirect_url: &str,
) -> Result<String>
{
  let (sender_token_channel, reciever_token_channel) = channel::<String>();
  let (sender_shutdown_channel, reciever_shutdown_channel) = channel::<()>();

  let client_clone = client.clone();
  let url_clone = redirect_url.to_string();

  let server_task = tokio::spawn(async move {
    if let Err(e) = run_proxy_server(
      &url_clone,
      &client_clone,
      sender_token_channel,
      reciever_shutdown_channel,
    )
    .await
    {
      error!("Proxy server encountered an error: {}", e);
    }
  });

  let token = reciever_token_channel
    .await
    .map_err(|_| anyhow::anyhow!("Server dropped"))?;

  info!("Token received succefully! Stopping server...");
  let _ = sender_shutdown_channel.send(());
  let _ = server_task.await;

  Ok(token)
}
