use worker::*;

mod auth;
pub mod error;
mod handlers;
pub(crate) mod helpers;
mod router;

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let log_level = env
        .var("LOG_LEVEL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "info".to_string());
    kartoteka_logging::init_cf(&log_level);
    let request_id = req
        .headers()
        .get("X-Request-Id")
        .ok()
        .flatten()
        .unwrap_or_else(|| nanoid::nanoid!());

    let user_id = req
        .headers()
        .get("X-User-Id")
        .ok()
        .flatten()
        .unwrap_or_default();

    let span = tracing::info_span!("request",
        request_id = %request_id,
        method = %req.method(),
        path = %req.path(),
        user_id = %user_id,
    );

    let response = {
        let _guard = span.enter();
        router::handle(req, env).await
    };

    match &response {
        Ok(resp) => {
            let _guard = span.enter();
            tracing::info!(status = resp.status_code(), "request completed");
        }
        Err(e) => {
            let _guard = span.enter();
            tracing::error!(error = %e, "request failed");
        }
    }

    response
}
