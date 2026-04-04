mod archive;
mod crud;
mod features;
mod pin;
mod placement;

pub use archive::{list_archived, toggle_archive};
pub use crud::{create, create_sublist, delete, get_one, list_all, list_sublists, reset, update};
pub use features::{add_feature, remove_feature};
pub use pin::toggle_pin;
pub use placement::{move_list, set_placement};

use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

pub(super) const LIST_SELECT: &str = "\
    SELECT l.id, l.user_id, l.name, l.description, l.list_type, \
    l.parent_list_id, l.position, l.archived, l.container_id, l.pinned, l.last_opened_at, \
    l.created_at, l.updated_at, \
    COALESCE((SELECT json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config))) \
    FROM list_features lf WHERE lf.list_id = l.id), '[]') as features \
    FROM lists l";

pub(super) fn placement_filter(
    parent_list_id: Option<&str>,
    container_id: Option<&str>,
) -> (&'static str, Vec<JsValue>) {
    match (parent_list_id, container_id) {
        (Some(parent_id), None) => ("parent_list_id = ?1", vec![parent_id.into()]),
        (None, Some(container_id)) => (
            "parent_list_id IS NULL AND container_id = ?1",
            vec![container_id.into()],
        ),
        (None, None) => ("parent_list_id IS NULL AND container_id IS NULL", vec![]),
        (Some(_), Some(_)) => unreachable!("validated earlier"),
    }
}

pub(super) async fn ensure_parent_list_target(
    d1: &D1Database,
    parent_id: &str,
    user_id: &str,
) -> Result<bool> {
    Ok(d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2 AND parent_list_id IS NULL")
        .bind(&[parent_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

pub(super) async fn list_has_sublists(d1: &D1Database, list_id: &str) -> Result<bool> {
    Ok(d1
        .prepare("SELECT 1 FROM lists WHERE parent_list_id = ?1 LIMIT 1")
        .bind(&[list_id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

pub(super) async fn create_list_from_request(
    d1: &D1Database,
    user_id: &str,
    body: CreateListRequest,
) -> Result<Response> {
    if let Err(code) = body.validate_placement() {
        return json_error(code, 400);
    }

    if let Some(ref parent_id) = body.parent_list_id
        && !ensure_parent_list_target(d1, parent_id, user_id).await?
    {
        return json_error("list_not_found", 404);
    }

    if let Some(ref container_id) = body.container_id
        && !check_ownership(d1, "containers", container_id, user_id).await?
    {
        return json_error("container_not_found", 404);
    }

    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let list_type_str = serde_json::to_value(&body.list_type)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();
    let (filter, params) =
        placement_filter(body.parent_list_id.as_deref(), body.container_id.as_deref());
    let position = next_position(d1, "lists", filter, &params).await?;
    let parent_val = opt_str_to_js(&body.parent_list_id);
    let container_val = opt_str_to_js(&body.container_id);

    d1.prepare(
        "INSERT INTO lists (id, user_id, name, list_type, parent_list_id, container_id, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&[
        id.clone().into(),
        user_id.into(),
        body.name.clone().into(),
        list_type_str.into(),
        parent_val,
        container_val,
        position.into(),
    ])?
    .run()
    .await?;

    let features = body
        .features
        .unwrap_or_else(|| body.list_type.default_features());
    for feature in &features {
        let config_str = feature.config.to_string();
        d1.prepare("INSERT INTO list_features (list_id, feature_name, config) VALUES (?1, ?2, ?3)")
            .bind(&[
                id.clone().into(),
                feature.name.clone().into(),
                config_str.into(),
            ])?
            .run()
            .await?;
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create list"))?;

    Ok(Response::from_json(&list)?.with_status(201))
}
