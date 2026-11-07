use kartoteka_shared::types::{Comment, CommentsPayload};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, format_datetime_in_tz, require_user},
    kartoteka_domain as domain,
};

#[cfg(feature = "ssr")]
pub(crate) fn domain_comment_to_shared(c: domain::comments::Comment, tz: &str) -> Comment {
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
) -> Result<CommentsPayload, ServerFnError> {
    let (pool, user) = require_user().await?;
    let (comments, prefs) = tokio::try_join!(
        domain::comments::list_for_entity(&pool, &user.id, &entity_type, &entity_id),
        domain::preferences::get(&pool, &user.id),
    )
    .sfn()?;
    Ok(CommentsPayload {
        comments: comments
            .into_iter()
            .map(|c| domain_comment_to_shared(c, &prefs.timezone))
            .collect(),
        current_user_id: user.id,
    })
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
    let (pool, user) = require_user().await?;
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
    .sfn()?;
    let prefs = domain::preferences::get(&pool, &user.id).await.sfn()?;
    Ok(domain_comment_to_shared(comment, &prefs.timezone))
}

#[server(prefix = "/leptos")]
pub async fn remove_comment(comment_id: String) -> Result<(), ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::comments::delete(&pool, &user.id, &comment_id)
        .await
        .sfn()?;
    Ok(())
}
