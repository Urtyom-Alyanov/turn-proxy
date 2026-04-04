use std::sync::Arc;

use anyhow::Result;
use reqwest::Url;
use tokio::sync::{Mutex, oneshot};
use tracing::info;

use crate::providers::vk::captcha_solve::{PROXY_ADDR, reverse_proxy};

pub async fn solve_captcha_via_proxy(redirect_uri: &str) -> Result<String>
{
  let target_url = Url::parse(&redirect_uri)?;
  let (tx, rx) = oneshot::channel();

  let ctx = Arc::new(reverse_proxy::ProxyContext {
    target_url,
    token_tx: Mutex::new(Some(tx)),
    http_client: reqwest::Client::builder()
      .redirect(reqwest::redirect::Policy::none())
      .gzip(true)
      .build()?,
  });

  let server_ctx = ctx.clone();
  let abort_handle = tokio::spawn(async move {
    if let Err(e) = reverse_proxy::run_proxy_server(server_ctx).await {
      tracing::error!("Proxy server error: {}", e);
    }
  });

  let url_open = format!("http://{}", PROXY_ADDR);
  info!("Opening browser for captcha: {}", url_open);
  let _ = open::that(url_open);

  let token = rx.await?;

  abort_handle.abort();

  Ok(token)
}
