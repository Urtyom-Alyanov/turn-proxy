use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_logger() -> WorkerGuard
{
  let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

  let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

  tracing_subscriber::registry()
    .with(filter)
    .with(fmt::layer().with_writer(non_blocking))
    .init();

  guard
}
