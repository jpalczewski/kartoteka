use kartoteka_shared::types::Relation;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, require_user},
    kartoteka_domain as domain,
};

#[cfg(feature = "ssr")]
fn domain_rel_to_shared(r: domain::relations::Relation) -> Relation {
    Relation {
        id: r.id,
        from_type: r.from_type,
        from_id: r.from_id,
        to_type: r.to_type,
        to_id: r.to_id,
        relation_type: r.relation_type,
        user_id: r.user_id,
        created_at: r.created_at,
        other_type: r.other_type,
        other_title: r.other_title,
        other_completed: r.other_completed,
    }
}

/// Fetch all relations for an entity (bidirectional).
/// entity_type: "item" | "list" | "container"
/// Each relation carries `other_title` (and `other_completed` for items) so the
/// UI can render names instead of ids.
#[server(prefix = "/leptos")]
pub async fn get_relations(
    entity_type: String,
    entity_id: String,
) -> Result<Vec<Relation>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let rels = domain::relations::get_for_entity(&pool, &user.id, &entity_type, &entity_id)
        .await
        .sfn()?;
    Ok(rels.into_iter().map(domain_rel_to_shared).collect())
}

/// Add a relation. Current entity is always "from".
#[server(prefix = "/leptos")]
pub async fn add_relation(
    from_type: String,
    from_id: String,
    to_type: String,
    to_id: String,
    relation_type: String,
) -> Result<Relation, ServerFnError> {
    let (pool, user) = require_user().await?;
    let rel = domain::relations::create(
        &pool,
        &user.id,
        &from_type,
        &from_id,
        &to_type,
        &to_id,
        &relation_type,
    )
    .await
    .sfn()?;
    Ok(domain_rel_to_shared(rel))
}

/// Remove a relation by id.
#[server(prefix = "/leptos")]
pub async fn remove_relation(relation_id: String) -> Result<(), ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::relations::delete(&pool, &user.id, &relation_id)
        .await
        .sfn()?;
    Ok(())
}
