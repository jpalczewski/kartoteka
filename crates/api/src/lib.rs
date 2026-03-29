use tracing::Instrument;
use worker::*;

mod auth;
pub mod error;
mod handlers;
pub(crate) mod helpers;
mod router;

#[event(start)]
fn start() {
    kartoteka_logging::init_cf();
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
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

    let response = router::handle(req, env).instrument(span.clone()).await;

    match &response {
        Ok(resp) => {
            let _enter = span.enter();
            tracing::info!(status = resp.status_code(), "request completed");
        }
        Err(e) => {
            let _enter = span.enter();
            tracing::error!(error = %e, "request failed");
        }
    }

    response
}
