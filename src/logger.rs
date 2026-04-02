use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

pub fn init_logger() {
    // set log level from RUST_LOG env var
    let env_filter = EnvFilter::try_from_default_env();

    // create subscriber env filter
    let subscriber_env_filter =
        env_filter.unwrap_or_else(|_| EnvFilter::new("info,sqlx::migrate=warn"));

    // create stdout layer
    let stdout_log = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time()
        .compact()
        .with_ansi_sanitization(false)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry().with(stdout_log).with(subscriber_env_filter).init();
}
