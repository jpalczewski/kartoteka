use worker::*;

use crate::auth;
use crate::handlers::{items, lists};

fn cors_headers() -> Headers {
    let headers = Headers::new();
    // TODO: restrict to actual domain in production
    let _ = headers.set("Access-Control-Allow-Origin", "*");
    let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
    let _ = headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization");
    headers
}

pub async fn handle(req: Request, env: Env) -> Result<Response> {
    let cors = cors_headers();

    if req.method() == Method::Options {
        return Ok(Response::empty()?.with_headers(cors));
    }

    let path = req.path();
    if path == "/api/health" {
        return Ok(Response::ok("ok")?.with_headers(cors));
    }

    // Validate Hanko session
    if let Err(e) = auth::validate_session(&req).await {
        let body = serde_json::json!({ "error": e.to_string() });
        return Ok(Response::from_json(&body)?
            .with_status(401)
            .with_headers(cors));
    }

    let router = Router::new();
    let response = router
        // Lists
        .get_async("/api/lists", lists::list_all)
        .post_async("/api/lists", lists::create)
        .get_async("/api/lists/:id", lists::get_one)
        .put_async("/api/lists/:id", lists::update)
        .delete_async("/api/lists/:id", lists::delete)
        // Items
        .get_async("/api/lists/:list_id/items", items::list_all)
        .post_async("/api/lists/:list_id/items", items::create)
        .put_async("/api/lists/:list_id/items/:id", items::update)
        .delete_async("/api/lists/:list_id/items/:id", items::delete)
        .run(req, env)
        .await?;

    Ok(response.with_headers(cors))
}
