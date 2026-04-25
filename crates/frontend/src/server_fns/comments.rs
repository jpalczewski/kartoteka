use kartoteka_shared::types::Comment;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::format_datetime_in_tz, axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend, kartoteka_domain as domain, sqlx::SqlitePool,
};

#[cfg(feature = "ssr")]
fn domain_comment_to_shared(c: domain::comments::Comment, tz: &str) -> Comment {
    Comment {
        id: c.id,
        entity_type: c.entity_type,
        entity_id: c.entity_id,
        content: c.content,
        author_type: c.author_type,
        author_name: c.author_name,
        user_id: c.user_id,
        created_at: format_datetime_in_tz(&c.created_at, tz),
        updated_at: c.updated_at,
    }
}

#[server(prefix = "/leptos")]
pub async fn get_comments(
    entity_type: String,
    entity_id: String,
) -> Result<Vec<Comment>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let (comments, prefs) = tokio::try_join!(
        domain::comments::list_for_entity(&pool, &user.id, &entity_type, &entity_id),
        domain::preferences::get(&pool, &user.id),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(comments
        .into_iter()
        .map(|c| domain_comment_to_shared(c, &prefs.timezone))
        .collect())
}

#[server(prefix = "/leptos")]
pub async fn add_comment(
    entity_type: String,
    entity_id: String,
    content: String,
) -> Result<Comment, ServerFnError> {
    if content.trim().is_empty() {
        return Err(ServerFnError::new("comment cannot be empty".to_string()));
    }
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let comment = domain::comments::create(
        &pool,
        &user.id,
        &entity_type,
        &entity_id,
        &content,
        "user",
        None,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    let prefs = domain::preferences::get(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_comment_to_shared(comment, &prefs.timezone))
}

#[server(prefix = "/leptos")]
pub async fn remove_comment(comment_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::comments::delete(&pool, &user.id, &comment_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(prefix = "/leptos")]
pub async fn get_current_user_id() -> Result<String, ServerFnError> {
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    Ok(user.id)
}
