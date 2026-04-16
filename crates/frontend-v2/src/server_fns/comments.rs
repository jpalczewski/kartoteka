use kartoteka_shared::types::Comment;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession, kartoteka_auth::KartotekaBackend, kartoteka_domain as domain,
    sqlx::SqlitePool,
};

#[cfg(feature = "ssr")]
fn domain_comment_to_shared(c: domain::comments::Comment) -> Comment {
    Comment {
        id: c.id,
        entity_type: c.entity_type,
        entity_id: c.entity_id,
        content: c.content,
        author_type: c.author_type,
        author_name: c.author_name,
        user_id: c.user_id,
        created_at: c.created_at,
        updated_at: c.updated_at,
    }
}

/// Fetch all comments for an entity (entity_type: "item" | "list" | "container").
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
    let comments = domain::comments::list_for_entity(&pool, &user.id, &entity_type, &entity_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(comments.into_iter().map(domain_comment_to_shared).collect())
}

/// Add a comment to an entity. Author type is always "user" from browser.
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
    Ok(domain_comment_to_shared(comment))
}

/// Delete a comment by id. Only the comment's author can delete it (enforced by db).
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

/// Return the current user's id (for client-side delete button visibility).
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
