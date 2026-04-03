use std::sync::Arc;

use axum::{
  Router,
  body::Body,
  extract::{OriginalUri, Query, State},
  http::{HeaderMap, Method, Response, StatusCode, header},
  response::{Html, IntoResponse},
  routing::{get, post},
};
use tokio::sync::{Mutex, oneshot};
use tracing::info;
use url::Url;

use crate::providers::vk::captcha_solve::PROXY_ADDR;

const INJECT_SCRIPT: &str = include_str!("inject.html");

pub struct ProxyContext
{
  pub target_url: Url,
  pub token_tx: Mutex<Option<oneshot::Sender<String>>>,
  pub http_client: reqwest::Client,
}

pub async fn run_proxy_server(ctx: Arc<ProxyContext>) -> anyhow::Result<()>
{
  let app = Router::new()
    .route("/", get(proxy_handler).post(proxy_handler))
    .route("/local-captcha-result", post(captcha_result_handler))
    .route("/generic_proxy", get(generic_proxy_handler))
    .fallback(proxy_handler)
    .with_state(ctx);

  let listener = tokio::net::TcpListener::bind(PROXY_ADDR).await?;
  info!("Captcha proxy server listening on http://{}", PROXY_ADDR);

  axum::serve(listener, app).await?;
  Ok(())
}

async fn proxy_handler(
  State(ctx): State<Arc<ProxyContext>>,
  method: Method,
  headers: HeaderMap,
  OriginalUri(uri): OriginalUri,
  body: Body,
) -> impl IntoResponse
{
  let mut target = ctx.target_url.clone();
  if uri.path() != "/" {
    target.set_path(uri.path());
    target.set_query(uri.query());
  }

  let mut req_builder = ctx.http_client.request(method, target.as_str());

  for (key, value) in headers.iter() {
    if key == header::HOST {
      continue;
    }
    req_builder = req_builder.header(key, value);
  }

  let reqwest_body = reqwest::Body::wrap_stream(body.into_data_stream());

  let vk_resp = match req_builder.body(reqwest_body).send().await {
    Ok(r) => r,
    Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
  };

  let status = vk_resp.status();
  let mut res_headers = HeaderMap::new();
  for (k, v) in vk_resp.headers().iter() {
    res_headers.insert(k, v.clone());
  }

  if status.is_redirection() {
    if let Some(loc_val) = res_headers.get_mut(header::LOCATION) {
      if let Ok(loc_str) = loc_val.to_str() {
        let target_origin = format!(
          "{}://{}",
          ctx.target_url.scheme(),
          ctx.target_url.host_str().unwrap_or("")
        );
        let new_loc =
          loc_str.replace(&target_origin, &format!("http://{}", PROXY_ADDR));
        *loc_val =
          header::HeaderValue::from_str(&new_loc).unwrap_or(loc_val.clone());
      }
    }
  }

  let content_type = res_headers
    .get(header::CONTENT_TYPE)
    .and_then(|v| v.to_str().ok())
    .unwrap_or("");

  if content_type.contains("text/html") {
    let text = vk_resp.text().await.unwrap_or_default();
    let modified_html =
      text.replace("</head>", &format!("{}{}", INJECT_SCRIPT, "</head>"));

    res_headers.remove(header::CONTENT_LENGTH);
    res_headers.remove(header::CONTENT_ENCODING); // Текст уже разжат reqwest-ом

    let mut response = Html(modified_html).into_response();
    *response.headers_mut() = res_headers;
    *response.status_mut() = status;
    response
  } else {
    let bytes = vk_resp.bytes().await.unwrap_or_default();
    let mut response = Response::new(Body::from(bytes));
    *response.headers_mut() = res_headers;
    *response.status_mut() = status;
    response
  }
}

async fn generic_proxy_handler(
  State(ctx): State<Arc<ProxyContext>>,
  Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse
{
  let url = params.get("proxy_url").cloned().unwrap_or_default();
  if url.is_empty() {
    return StatusCode::BAD_REQUEST.into_response();
  }

  let resp = match ctx.http_client.get(url).send().await {
    Ok(r) => r,
    Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
  };

  let mut rb = Response::builder().status(resp.status());
  for (k, v) in resp.headers() {
    rb = rb.header(k, v);
  }
  rb.body(Body::from(resp.bytes().await.unwrap_or_default()))
    .unwrap()
    .into_response()
}

#[derive(serde::Deserialize)]
struct TokenPayload
{
  token: String,
}

async fn captcha_result_handler(
  State(ctx): State<Arc<ProxyContext>>,
  axum::Json(payload): axum::Json<TokenPayload>,
) -> impl IntoResponse
{
  let mut guard = ctx.token_tx.lock().await;
  if let Some(tx) = guard.take() {
    let _ = tx.send(payload.token);
  }
  "ok"
}
