use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
  #[arg(long, default_value = "0.0.0.0:56000")]
  pub listening_on: Option<String>,

  #[arg(long)]
  pub proxy_into: Option<String>,

  #[arg(long)]
  pub no_config: bool,

  #[arg(long, default_value = "/etc/turn-proxy/server/config.toml")]
  pub config: String,
}