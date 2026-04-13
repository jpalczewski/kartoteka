use kartoteka_shared::types::{HomeData, List, ListFeature, Tag};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
};

/// Convert domain::lists::List to shared::types::List.
/// Used in server functions returning list data.
#[cfg(feature = "ssr")]
pub(crate) fn domain_list_to_shared(l: domain::lists::List) -> List {
    List {
        id: l.id,
        user_id: l.user_id,
        name: l.name,
        icon: l.icon,
        description: l.description,
        list_type: l.list_type,
        parent_list_id: l.parent_list_id,
        position: l.position,
        archived: l.archived,
        container_id: l.container_id,
        pinned: l.pinned,
        last_opened_at: l.last_opened_at,
        created_at: l.created_at,
        updated_at: l.updated_at,
        features: l
            .features
            .into_iter()
            .map(|f| ListFeature {
                feature_name: f.feature_name,
                config: f.config,
            })
            .collect(),
    }
}

/// Convert domain::tags::Tag to shared::types::Tag.
#[cfg(feature = "ssr")]
pub(crate) fn domain_tag_to_shared(t: domain::tags::Tag) -> Tag {
    Tag {
        id: t.id,
        user_id: t.user_id,
        name: t.name,
        icon: t.icon,
        color: t.color,
        parent_tag_id: t.parent_tag_id,
        tag_type: t.tag_type,
        metadata: t.metadata,
        created_at: t.created_at,
    }
}

/// Home page data: 6 sections (pinned/recent/root for both containers and lists).
#[server(prefix = "/leptos")]
pub async fn get_home_data() -> Result<HomeData, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::home::query(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Archived lists for the home page archive section.
#[server(prefix = "/leptos")]
pub async fn get_archived_lists() -> Result<Vec<List>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let lists = domain::lists::list_archived(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(lists.into_iter().map(domain_list_to_shared).collect())
}
