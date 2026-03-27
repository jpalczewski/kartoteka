use worker::*;

use crate::auth;
use crate::handlers::{containers, items, lists, tags};

fn cors_headers() -> Headers {
    let headers = Headers::new();
    // TODO: restrict to actual domain in production
    let _ = headers.set("Access-Control-Allow-Origin", "*");
    let _ = headers.set(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, PATCH, DELETE, OPTIONS",
    );
    let _ = headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    );
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

    let user_id = if let Some(uid) = auth::dev_bypass_user_id(&env) {
        uid
    } else {
        match auth::validate_session(&req).await {
            Ok(uid) => uid,
            Err(e) => {
                let body = serde_json::json!({ "error": e.to_string() });
                return Ok(Response::from_json(&body)?
                    .with_status(401)
                    .with_headers(cors));
            }
        }
    };

    let router = Router::with_data(user_id);
    let response = router
        // Home
        .get_async("/api/home", containers::home)
        // Containers
        .get_async("/api/containers", containers::list_all)
        .post_async("/api/containers", containers::create)
        .get_async("/api/containers/:id", containers::get_one)
        .put_async("/api/containers/:id", containers::update)
        .delete_async("/api/containers/:id", containers::delete)
        .get_async("/api/containers/:id/children", containers::get_children)
        .patch_async("/api/containers/:id/move", containers::move_container)
        .patch_async("/api/containers/:id/pin", containers::toggle_pin)
        // Lists
        .get_async("/api/lists", lists::list_all)
        .post_async("/api/lists", lists::create)
        .get_async("/api/lists/archived", lists::list_archived)
        .patch_async("/api/lists/:id/archive", lists::toggle_archive)
        .post_async("/api/lists/:id/reset", lists::reset)
        .get_async("/api/lists/:id", lists::get_one)
        .put_async("/api/lists/:id", lists::update)
        .delete_async("/api/lists/:id", lists::delete)
        // Sublists
        .get_async("/api/lists/:id/sublists", lists::list_sublists)
        .post_async("/api/lists/:id/sublists", lists::create_sublist)
        // List features
        .post_async("/api/lists/:id/features/:name", lists::add_feature)
        .delete_async("/api/lists/:id/features/:name", lists::remove_feature)
        // Items
        .get_async("/api/lists/:list_id/items", items::list_all)
        .post_async("/api/lists/:list_id/items", items::create)
        .put_async("/api/lists/:list_id/items/:id", items::update)
        .delete_async("/api/lists/:list_id/items/:id", items::delete)
        // Cross-list queries
        .get_async("/api/items/by-date", items::by_date)
        .get_async("/api/items/calendar", items::calendar)
        // List container + pin
        .patch_async("/api/lists/:id/container", lists::move_list)
        .patch_async("/api/lists/:id/pin", lists::toggle_pin)
        // Item move
        .patch_async("/api/items/:id/move", items::move_item)
        // Tags CRUD
        .get_async("/api/tags", tags::list_all)
        .post_async("/api/tags", tags::create)
        .get_async("/api/tags/:id/items", tags::tag_items)
        .put_async("/api/tags/:id", tags::update)
        .delete_async("/api/tags/:id", tags::delete)
        .post_async("/api/tags/:id/merge", tags::merge)
        // Tag assignments
        .post_async("/api/items/:item_id/tags", tags::assign_to_item)
        .delete_async("/api/items/:item_id/tags/:tag_id", tags::remove_from_item)
        .post_async("/api/lists/:list_id/tags", tags::assign_to_list)
        .delete_async("/api/lists/:list_id/tags/:tag_id", tags::remove_from_list)
        // Tag link queries
        .get_async("/api/tag-links/items", tags::all_item_tag_links)
        .get_async("/api/tag-links/lists", tags::all_list_tag_links)
        .run(req, env)
        .await?;

    Ok(response.with_headers(cors))
}
