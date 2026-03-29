pub mod error;

pub use error::{ApiError, ApiResult, created_response, into_response, no_content_response, ok_response};

#[cfg(feature = "cf")]
pub fn init_cf() {
    use tracing_subscriber::fmt::format::Pretty;
    use tracing_subscriber::fmt::time::UtcTime;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::prelude::*;
    use tracing_web::{MakeConsoleWriter, performance_layer};

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(MakeConsoleWriter);
    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt_layer)
        .with(perf_layer)
        .try_init();
}

#[cfg(feature = "axum")]
pub fn init_axum() {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .init();
}
