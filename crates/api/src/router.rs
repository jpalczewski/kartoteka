use worker::*;

use crate::auth;
use crate::handlers::{admin, containers, items, lists, me, preferences, public, settings, tags};

pub async fn handle(req: Request, env: Env) -> Result<Response> {
    let path = req.path();
    if path == "/api/health" {
        return Response::ok("ok");
    }

    // Public routes — no auth required
    if path.starts_with("/api/public/") {
        let public_router = Router::with_data(String::new());
        return public_router
            .get_async("/api/public/registration-mode", public::get_registration_mode)
            .post_async("/api/public/validate-invite", public::validate_invite)
            .run(req, env)
            .await;
    }

    let user_id = if let Some(uid) = auth::dev_bypass_user_id(&env) {
        uid
    } else {
        auth::user_id_from_gateway(&req)?
    };

    let router = Router::with_data(user_id);
    router
        // Me
        .get_async("/api/me", me::get_me)
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
        .get_async("/api/lists/:list_id/items/:id", items::get_one)
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
        // Preferences
        .get_async("/api/preferences", preferences::get_preferences)
        .put_async("/api/preferences", preferences::put_preferences)
        // Settings
        .get_async("/api/settings", settings::list_all)
        .put_async("/api/settings/:key", settings::upsert)
        .delete_async("/api/settings/:key", settings::delete)
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
        // Admin — instance settings
        .get_async("/api/admin/instance-settings", admin::list_settings)
        .put_async("/api/admin/instance-settings/:key", admin::update_setting)
        // Admin — invitation codes
        .get_async("/api/admin/invitation-codes", admin::list_codes)
        .post_async("/api/admin/invitation-codes", admin::create_code)
        .delete_async("/api/admin/invitation-codes/:id", admin::delete_code)
        .run(req, env)
        .await
}
