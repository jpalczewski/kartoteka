use worker::*;

mod auth;
mod handlers;
pub(crate) mod helpers;
mod router;

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    router::handle(req, env).await
}
