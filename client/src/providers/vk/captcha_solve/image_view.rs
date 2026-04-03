use std::sync::Arc;

use axum::{
  Router,
  extract::{Query, State},
  response::{Html, IntoResponse},
  routing::get,
};
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::info;

use crate::providers::vk::captcha_solve::IMAGE_SERVER_ADDR;

const IMAGE_FORM: &str = include_str!("./image_captcha_form.html");

pub struct ImageContext
{
  pub captcha_img: String,
  pub result_tx: Mutex<Option<oneshot::Sender<String>>>,
}

#[derive(Deserialize)]
pub struct SolveParams
{
  pub key: String,
}

pub async fn solve_captcha_via_image(
  captcha_img: &str,
) -> anyhow::Result<String>
{
  let (tx, rx) = oneshot::channel();

  let ctx = Arc::new(ImageContext {
    captcha_img: captcha_img.to_owned(),
    result_tx: Mutex::new(Some(tx)),
  });

  let server_ctx = ctx.clone();
  let abort_handle = tokio::spawn(async move {
    if let Err(e) = run_image_server(server_ctx).await {
      tracing::error!("Image captcha server error: {}", e);
    }
  });

  let local_url = "http://127.0.0.1:8765/";
  info!("CAPTCHA_REQUIRED: {}", local_url);
  let _ = open::that(local_url);

  let key = rx.try_recv().unwrap_or_else(|e| panic!("{}", e));

  abort_handle.abort();

  Ok(key)
}

pub async fn run_image_server(ctx: Arc<ImageContext>) -> anyhow::Result<()>
{
  let app = Router::new()
    .route("/", get(show_form_handler))
    .route("/solve", get(solve_handler))
    .with_state(ctx);

  let listener = tokio::net::TcpListener::bind(IMAGE_SERVER_ADDR).await?;
  axum::serve(listener, app).await?;
  Ok(())
}

async fn show_form_handler(
  State(ctx): State<Arc<ImageContext>>,
) -> impl IntoResponse
{
  Html(IMAGE_FORM.replace("{image}", &ctx.captcha_img))
}

async fn solve_handler(
  State(ctx): State<Arc<ImageContext>>,
  Query(params): Query<SolveParams>,
) -> impl IntoResponse
{
  let mut guard = ctx.result_tx.lock().await;
  if let Some(tx) = guard.take() {
    let _ = tx.send(params.key);
  }

  Html(
    "<h2>Готово! Возвращайтесь в консоль.</h2><script>setTimeout(window.close, 1000);</script>",
  )
}
