use crate::config::args::Args;
use crate::config::configuration::{AppConfig, CommonConfig};

use std::fs;
use anyhow::Result;
use anyhow::Context;
use clap::Parser;

pub fn init_config() -> Result<AppConfig> {
  let args = Args::parse();

  let config = if !args.no_config {
    let content = fs::read_to_string(&args.config)
      .with_context(|| format!("[ERROR] read configuration file error: {}", args.config))?;
    toml::from_str::<AppConfig>(&content)
      .context(format!("[ERROR] TOML configuration parse error (path: {})", args.config))?
  } else {
    AppConfig::default()
  };

  let final_listen = args.listening_on
    .or(config.common.listening_on)
    .unwrap_or_else(|| "0.0.0.0:56000".to_string());

  let final_proxy = args.proxy_into
    .or(config.common.proxy_into)
    .context("[ERROR] proxy_into address is missing")?;

  Ok(AppConfig {
    common: CommonConfig {
      listening_on: final_listen.into(),
      proxy_into: final_proxy.into()
    }
  })
}