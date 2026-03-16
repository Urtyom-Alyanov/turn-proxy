pub mod logging;
pub mod config;
pub mod proxy_process;
pub mod dtls;

use crate::logging::init_logging;
use crate::config::init_configuration::init_config;
use crate::dtls::dtls_configure;
use crate::proxy_process::listening::listening;

use anyhow::{Result};

#[tokio::main]
async fn main() -> Result<()> {
  init_logging();
  let config = init_config()?;
  let dtls_config = dtls_configure()?;

  listening(config, dtls_config).await?;

  Ok(())
}