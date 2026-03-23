use worker::*;

use crate::handlers::{items, lists};

fn cors_headers() -> Headers {
    let headers = Headers::new();
    let _ = headers.set("Access-Control-Allow-Origin", "*");
    let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
    let _ = headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization");
    headers
}

pub async fn handle(req: Request, env: Env) -> Result<Response> {
    let cors = cors_headers();

    if req.method() == Method::Options {
        let mut resp = Response::empty()?;
        resp = resp.with_headers(cors);
        return Ok(resp);
    }

    let router = Router::new();
    let response = router
        // Health
        .get("/api/health", |_, _| Response::ok("ok"))
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
