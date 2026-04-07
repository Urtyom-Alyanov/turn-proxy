
use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::{post, get, any}};
use reqwest::{Client};
use tokio::sync::Mutex;

use crate::providers::vk::captcha_solve::PROXY_ADDR;

struct ProxyContext {
  redirect_url: reqwest::Url,
  http_client: Client,
  token: Mutex<Option<String>>,
}

/// Запускает локальный HTTP сервер, который отображает страницу для ввода капчи и слушает результат.
async fn run_proxy_server(redirect_url: &str, client: &Client) -> Result<()>
{
  let ctx = Arc::new(ProxyContext {
    redirect_url: reqwest::Url::parse(redirect_url).unwrap(),
    http_client: client.clone(),
    token: Mutex::new(None),
  });

  let router = Router::new()
    .route("/", get(captcha_user_input_handler))
    .route("/request", any(captcha_request_handler))
    .route("/submit", post(captcha_submit_handler))
    .with_state(ctx);
  
  let listener = tokio::net::TcpListener::bind(PROXY_ADDR).await?;

  axum::serve(listener, router).await?;

  Ok(())
}

/// Здесь, получается `success_token` от решения капчи, который мы потом записываем в `Mutex`
async fn captcha_submit_handler()
{

}

/// Обрабатывает запросы от VK API, перенаправляя их на страницу ввода капчи и ожидая решения от пользователя
/// 
/// Для ВК показывается USER_AGENT нашего прокси, который имитирует браузер, через который идёт подключение,
/// а не реального клиента, что позволяет обойти некоторые проверки и успешно решать капчи.
async fn captcha_request_handler()
{

}

/// Страница для ввода капчи пользователем (инжектится кастомный JS, который всё перенаправляет в `/request`, если в поле есть `success_token` то шлёт его в `/submit`)
async fn captcha_user_input_handler()
{

}

/// Основная функция для решения капчи через обратный прокси. Запускает локальный сервер, который отображает страницу для ввода капчи и слушает результат.
/// После получения решения капчи, извлекает `session_token` и возвращает его. Этот токен можно использовать для повторной отправки запроса к VK API уже с решённой капчей.
pub async fn solve_via_reverse_proxy(
  client: &Client,
  redirect_url: &str,
) -> Result<String>
{
  run_proxy_server(redirect_url, client)
}