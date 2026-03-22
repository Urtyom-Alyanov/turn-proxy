use clap::Parser;

#[cfg(target_os = "windows")]
const DEFAULT_CONFIG_PATH: &str = ".\\config.toml";

#[cfg(target_os = "linux")]
const DEFAULT_CONFIG_PATH: &str = "/etc/turn-proxy/client/config.toml";

#[derive(Parser, Debug)]
pub struct Args {
  #[arg(long, short)]
  pub listening_on: Option<String>,

  #[arg(long, short)]
  pub proxy_into: Option<String>,

  #[arg(long, short)]
  pub no_config: bool,

  #[arg(long, short, default_value = DEFAULT_CONFIG_PATH)]
  pub config: String,
}