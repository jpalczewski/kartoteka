use kartoteka_shared::types::{ListTagLink, Tag};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::{domain_list_to_shared, domain_tag_to_shared},
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_db as db, kartoteka_domain as domain,
    sqlx::SqlitePool,
};

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

/// Create a new tag for the current user.
#[server(prefix = "/leptos")]
pub async fn create_tag(
    name: String,
    icon: Option<String>,
    color: Option<String>,
    tag_type: Option<String>,
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
            parent_tag_id: None,
            tag_type,
            metadata: None,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
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
