use std::{net::IpAddr, sync::Arc};

use anyhow::Result;
use axum::http::HeaderValue;
use reqwest::{Client, dns::Resolve};

use crate::inbound::user_agent::{Browser, Os, UserAgent};

/// Создаётся клиент с определённым DNS-резолвером и IP интерфейсом
pub fn create_client(
  addr_interface: IpAddr,
  resolver: Arc<dyn Resolve>,
  user_agent: &UserAgent,
) -> Result<Client>
{
  let mut headers = reqwest::header::HeaderMap::new();

  if user_agent.chromium_based {
    let major_version = user_agent.major_version;

    let brand_name = match user_agent.browser {
      Browser::Chrome => "Google Chrome",
      Browser::Edge => "Microsoft Edge",
      Browser::Opera => "Opera",
      Browser::YandexBrowser => "Yandex Browser",
      _ => "Chromium",
    };

    let platform = match user_agent.os {
      Os::Windows => "Windows",
      Os::MacintoshIntel => "macOS",
      _ => "Linux",
    };

    let sec_ch_ua = format!(
      r#""{}";v="{}", "Chromium";v="{}", "Not-A.Brand";v="24""#,
      brand_name, major_version, major_version
    );

    headers.insert("sec-ch-ua", HeaderValue::from_str(&sec_ch_ua)?);
    headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    headers.insert(
      "sec-ch-ua-platform",
      HeaderValue::from_str(&format!(r#""{}""#, platform))?,
    );
    headers.insert(
      "sec-ch-ua-full-version",
      HeaderValue::from_str(&user_agent.full_version)?,
    );
    headers.insert(
      "sec-ch-ua-platform-version",
      HeaderValue::from_static(r#""""#),
    );
    headers.insert("sec-ch-ua-model", HeaderValue::from_static(r#""""#));
    headers.insert("sec-ch-ua-arch", HeaderValue::from_static(r#""x86""#));
    headers.insert("sec-ch-ua-bitness", HeaderValue::from_static(r#""64""#));
    headers.insert("sec-ch-ua-wow64", HeaderValue::from_static("?0"));
    headers.insert(
      "sec-ch-ua-full-version-list",
      HeaderValue::from_str(&format!(
        r#""{}";v="{}", "Chromium";v="{}", "Not-A.Brand";v="24""#,
        brand_name, user_agent.full_version, user_agent.full_version
      ))?,
    );
    headers.insert(
      "sec-ch-ua-form-factors",
      HeaderValue::from_static("Desktop"),
    );

    headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
    headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
    headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-site"));
  };

  headers.insert(
    "Accept-Language",
    HeaderValue::from_static("ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7"),
  );

  let builder = reqwest::ClientBuilder::new()
    .no_proxy()
    .cookie_store(true)
    .hickory_dns(true)
    .user_agent(user_agent.value.clone())
    .default_headers(headers)
    .local_address(addr_interface)
    .dns_resolver(resolver);

  let client = builder.build()?;

  Ok(client)
}
