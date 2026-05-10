use kartoteka_shared::types::{DateItem, ItemTagLink, ListTagLink, Tag};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::{domain_list_to_shared, domain_tag_to_shared},
    crate::server_fns::items::domain_item_to_shared,
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_db as db, kartoteka_domain as domain,
    sqlx::SqlitePool,
};

/// Tags assigned to a specific item.
#[server(prefix = "/leptos")]
pub async fn get_item_tags(item_id: String) -> Result<Vec<Tag>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let tags = domain::tags::get_for_item(&pool, &user.id, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(tags.into_iter().map(domain_tag_to_shared).collect())
}

/// Assign a tag to an item. Enforces exclusive-type constraint (e.g. one "priority" per item).
#[server(prefix = "/leptos")]
pub async fn assign_tag_to_item(item_id: String, tag_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::assign_to_item(&pool, &user.id, &item_id, &tag_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Remove a tag from an item.
#[server(prefix = "/leptos")]
pub async fn remove_tag_from_item(item_id: String, tag_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::remove_from_item(&pool, &user.id, &item_id, &tag_id)
        .await
        .map(|_| ())
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// All tags for the current user (for tag filter bar and tag selectors).
#[server(prefix = "/leptos")]
pub async fn get_all_tags() -> Result<Vec<Tag>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let tags = domain::tags::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(tags.into_iter().map(domain_tag_to_shared).collect())
}

/// All list-tag associations for the current user.
#[server(prefix = "/leptos")]
pub async fn get_list_tag_links() -> Result<Vec<ListTagLink>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let pairs = db::tags::get_all_list_tag_links(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(pairs
        .into_iter()
        .map(|(list_id, tag_id)| ListTagLink { list_id, tag_id })
        .collect())
}

/// Assign a tag to a list (idempotent).
#[server(prefix = "/leptos")]
pub async fn assign_tag_to_list(list_id: String, tag_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::assign_to_list(&pool, &user.id, &list_id, &tag_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Remove a tag from a list.
#[server(prefix = "/leptos")]
pub async fn remove_tag_from_list(list_id: String, tag_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::remove_from_list(&pool, &user.id, &list_id, &tag_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Create a new tag for the current user. Pass `parent_tag_id=Some(id)` to nest under a parent.
#[server(prefix = "/leptos")]
pub async fn create_tag(
    name: String,
    icon: Option<String>,
    color: Option<String>,
    parent_tag_id: Option<String>,
) -> Result<Tag, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let tag = domain::tags::create(
        &pool,
        &user.id,
        &domain::tags::CreateTagRequest {
            name,
            icon,
            color,
            parent_tag_id,
            tag_type: None,
            metadata: None,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_tag_to_shared(tag))
}

/// Update any subset of tag fields. Uses `Option<Option<String>>` semantics where:
/// - `None` = don't change
/// - `Some(None)` (caller passes empty string) = clear field
/// - `Some(Some(value))` = set to value
///
/// For serialization simplicity, the wire protocol uses `Option<String>` for each field
/// (empty string means "clear"). `parent_tag_id` uses a separate `clear_parent` flag because
/// for IDs we can't distinguish "don't touch" from "set to empty" without a sentinel.
#[server(prefix = "/leptos")]
pub async fn update_tag(
    id: String,
    name: Option<String>,
    color: Option<String>,
    parent_tag_id: Option<String>,
    clear_parent: bool,
) -> Result<Tag, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let parent_field = if clear_parent {
        Some(None)
    } else {
        parent_tag_id.map(Some)
    };

    let tag = domain::tags::update(
        &pool,
        &user.id,
        &id,
        &domain::tags::UpdateTagRequest {
            name,
            icon: None,
            color: color.map(|c| if c.is_empty() { None } else { Some(c) }),
            parent_tag_id: parent_field,
            tag_type: None,
            metadata: None,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("tag not found".to_string()))?;
    Ok(domain_tag_to_shared(tag))
}

/// Merge `source_id` into `target_id`: reassign all links + children, then delete source.
#[server(prefix = "/leptos")]
pub async fn merge_tags(source_id: String, target_id: String) -> Result<Tag, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let tag = domain::tags::merge(&pool, &user.id, &source_id, &target_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_tag_to_shared(tag))
}

/// All item-tag associations for the current user. Used by filter chips and the tag-detail items view.
#[server(prefix = "/leptos")]
pub async fn get_all_item_tag_links() -> Result<Vec<ItemTagLink>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let pairs = db::tags::get_all_item_tag_links(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(pairs
        .into_iter()
        .map(|(item_id, tag_id)| ItemTagLink { item_id, tag_id })
        .collect())
}

/// Fetch items tagged with `tag_id`. When `recursive` is true, also includes items tagged with
/// any descendant of `tag_id`. Each returned item is enriched with the owning list's name.
#[server(prefix = "/leptos")]
pub async fn get_tag_items(
    tag_id: String,
    recursive: bool,
) -> Result<Vec<DateItem>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    // Resolve the set of tag ids to match.
    let mut target_tag_ids: std::collections::HashSet<String> =
        std::iter::once(tag_id.clone()).collect();
    if recursive {
        let all_tags = domain::tags::list_all(&pool, &user.id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        // BFS over parent_tag_id pointers.
        let mut stack = vec![tag_id.clone()];
        while let Some(current) = stack.pop() {
            for t in &all_tags {
                if t.parent_tag_id.as_deref() == Some(&current)
                    && target_tag_ids.insert(t.id.clone())
                {
                    stack.push(t.id.clone());
                }
            }
        }
    }

    // Fetch all item-tag links and all user items; filter in memory. For typical tag sizes this is
    // faster than an N-query-per-tag approach and simpler than dynamic SQL IN-lists.
    let all_links = db::tags::get_all_item_tag_links(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let matching_item_ids: std::collections::HashSet<String> = all_links
        .into_iter()
        .filter(|(_, tid)| target_tag_ids.contains(tid))
        .map(|(iid, _)| iid)
        .collect();
    if matching_item_ids.is_empty() {
        return Ok(Vec::new());
    }

    let all_items = domain::items::list_all_for_user(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let list_names: std::collections::HashMap<String, String> =
        all_lists.into_iter().map(|l| (l.id, l.name)).collect();

    Ok(all_items
        .into_iter()
        .filter(|i| matching_item_ids.contains(&i.id))
        .map(|item| {
            let list_name = list_names.get(&item.list_id).cloned().unwrap_or_default();
            DateItem {
                item: domain_item_to_shared(item),
                list_name,
            }
        })
        .collect())
}

/// Update tag color.
#[server(prefix = "/leptos")]
pub async fn update_tag_color(id: String, color: String) -> Result<Tag, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let tag = domain::tags::update(
        &pool,
        &user.id,
        &id,
        &domain::tags::UpdateTagRequest {
            name: None,
            icon: None,
            color: Some(Some(color)),
            parent_tag_id: None,
            tag_type: None,
            metadata: None,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("tag not found".to_string()))?;
    Ok(domain_tag_to_shared(tag))
}

/// Delete a tag by id.
#[server(prefix = "/leptos")]
pub async fn delete_tag(id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::delete(&pool, &user.id, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Fetch tag details + all lists linked to this tag.
#[server(prefix = "/leptos")]
pub async fn get_tag_detail_data(
    tag_id: String,
) -> Result<kartoteka_shared::types::TagDetailData, ServerFnError> {
    use kartoteka_shared::types::TagDetailData;
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let tag = domain::tags::get_one(&pool, &tag_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("tag not found".to_string()))?;

    let all_links = db::tags::get_all_list_tag_links(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let linked_ids: std::collections::HashSet<String> = all_links
        .into_iter()
        .filter(|(_, tid)| tid == &tag_id)
        .map(|(lid, _)| lid)
        .collect();

    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let linked_lists = all_lists
        .into_iter()
        .filter(|l| linked_ids.contains(&l.id))
        .map(domain_list_to_shared)
        .collect();

    Ok(TagDetailData {
        tag: domain_tag_to_shared(tag),
        linked_lists,
    })
}
