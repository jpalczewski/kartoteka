use crate::{DomainError, rules};
use kartoteka_db::types::{ContainerRow, DeleteTreeOutcome};
use kartoteka_db::{DbError, SqlitePool, containers as db_containers};
use kartoteka_shared::types::{
    Container, ContainerProgress, CreateContainerRequest, MoveContainerRequest,
    UpdateContainerRequest,
};
use sqlx::SqliteConnection;
use uuid::Uuid;

fn row_to_container(r: ContainerRow) -> Container {
    Container {
        id: r.id,
        user_id: r.user_id,
        name: r.name,
        icon: r.icon,
        color: r.color,
        description: r.description,
        status: r.status,
        parent_container_id: r.parent_container_id,
        position: r.position,
        pinned: r.pinned,
        archived: r.archived,
        last_opened_at: r.last_opened_at,
        location_id: r.location_id,
        created_at: r.created_at,
        updated_at: r.updated_at,
        ancestors: vec![],
    }
}

fn progress_row_to_domain(r: kartoteka_db::containers::ContainerProgressRow) -> ContainerProgress {
    ContainerProgress {
        total_lists: r.total_lists,
        total_items: r.total_items,
        completed_items: r.completed_items,
    }
}

/// List all containers for a user.
#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<Container>, DomainError> {
    let rows = db_containers::list_all(pool, user_id).await?;
    Ok(rows.into_iter().map(row_to_container).collect())
}

/// Get a single container by id (with ownership check).
#[tracing::instrument(skip(pool))]
pub async fn get_one(pool: &SqlitePool, id: &str, user_id: &str) -> Result<Container, DomainError> {
    db_containers::get_one(pool, id, user_id)
        .await?
        .map(row_to_container)
        .ok_or(DomainError::NotFound("container"))
}

/// Validate a container create request: the status value, and — when a parent
/// is given — that the parent exists, belongs to `user_id` and is a folder.
///
/// Takes an executor so the MCP batch can validate inside its own transaction,
/// where parents created earlier in the same batch are already visible.
pub async fn validate_create_container<'e, E>(
    executor: E,
    user_id: &str,
    req: &CreateContainerRequest,
) -> Result<(), DomainError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    rules::containers::validate_status(req.status.as_deref())?;
    rules::appearance::normalize_icon(req.icon.as_deref())?;
    rules::appearance::normalize_color(req.color.as_deref())?;

    if let Some(parent_id) = req.parent_container_id.as_deref() {
        let parent = db_containers::get_one(executor, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
        rules::containers::validate_hierarchy(parent.status.as_deref())?;
    }
    Ok(())
}

/// Insert an already-validated container inside a caller-owned transaction.
///
/// The request's icon/colour are normalized here (trim + "empty means none");
/// the format/length rules are enforced by [`validate_create_container`].
pub async fn insert_container_in(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    req: &CreateContainerRequest,
    position: i32,
) -> Result<Container, DomainError> {
    let normalized = CreateContainerRequest {
        name: req.name.clone(),
        icon: trim_optional(&req.icon),
        color: trim_optional(&req.color),
        description: req.description.clone(),
        status: req.status.clone(),
        parent_container_id: req.parent_container_id.clone(),
    };
    Ok(row_to_container(
        db_containers::insert_in_tx(conn, id, user_id, &normalized, position).await?,
    ))
}

/// Trim an optional string and turn an empty result into `None`.
fn trim_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Create a new container, validating hierarchy rules.
#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
) -> Result<Container, DomainError> {
    validate_create_container(pool, user_id, req).await?;

    // Phase 3: WRITE — compute position, then insert
    let position =
        db_containers::next_position(pool, user_id, req.parent_container_id.as_deref()).await?;
    let id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(DbError::Sqlx)?;
    let container = insert_container_in(&mut tx, &id, user_id, req, position).await?;
    tx.commit().await.map_err(DbError::Sqlx)?;
    Ok(container)
}

/// Update a container's metadata.
#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Container, DomainError> {
    rules::containers::validate_status(req.status.as_ref().and_then(|v| v.as_deref()))?;
    let icon = req
        .icon
        .as_ref()
        .map(|v| rules::appearance::normalize_icon(v.as_deref()))
        .transpose()?;
    let color = req
        .color
        .as_ref()
        .map(|v| rules::appearance::normalize_color(v.as_deref()))
        .transpose()?;
    let normalized = UpdateContainerRequest {
        name: req.name.clone(),
        icon,
        color,
        description: req.description.clone(),
        status: req.status.clone(),
        location_id: req.location_id.clone(),
    };
    crate::locations::ensure_owned(pool, &req.location_id, user_id).await?;
    db_containers::update(pool, id, user_id, &normalized)
        .await?
        .map(row_to_container)
        .ok_or(DomainError::NotFound("container"))
}

/// Change a container between folder (`None`) and project (`active` / `done` /
/// `paused`) states.
///
/// Turning a folder with sub-containers into a project is rejected: children may
/// only live under folders (same rule [`move_container`] applies to a target
/// parent), so that change would leave the hierarchy invalid.
#[tracing::instrument(skip(pool), fields(container_id = %id, status = ?status))]
pub async fn set_status(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    status: Option<&str>,
) -> Result<Container, DomainError> {
    rules::containers::validate_status(status)?;
    let current = get_one(pool, id, user_id).await?;
    if status.is_some() && current.status.is_none() {
        let children = db_containers::child_ids(pool, user_id, Some(id)).await?;
        if !children.is_empty() {
            return Err(DomainError::Validation("container_has_children"));
        }
    }
    if current.status.as_deref() == status {
        return Ok(current);
    }
    let req = UpdateContainerRequest {
        name: None,
        icon: None,
        color: None,
        description: None,
        status: Some(status.map(str::to_string)),
        location_id: None,
    };
    update(pool, id, user_id, &req).await
}

/// Set the icon and/or accent colour of a container (same patch convention as
/// [`crate::lists::set_appearance`]).
#[tracing::instrument(skip(pool, icon, color), fields(container_id = %id))]
pub async fn set_appearance(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    icon: Option<Option<String>>,
    color: Option<Option<String>>,
) -> Result<Container, DomainError> {
    if icon.is_none() && color.is_none() {
        return get_one(pool, id, user_id).await;
    }
    let req = UpdateContainerRequest {
        name: None,
        icon,
        color,
        description: None,
        status: None,
        location_id: None,
    };
    update(pool, id, user_id, &req).await
}

/// Preview what deleting this container would remove.
#[tracing::instrument(skip(pool))]
pub async fn delete_preview(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<kartoteka_shared::DeletePreview, DomainError> {
    let summary = db_containers::subtree_summary(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("container"))?;
    Ok(crate::delete_preview(summary))
}

/// Delete a container together with its whole subtree (subcontainers and their lists).
///
/// Deletion is refused while any list in the subtree still holds items. A subtree that
/// only has descendants requires `force`; forcing never bypasses the item check. The
/// item check is repeated inside the delete transaction, so a concurrent insert cannot
/// slip through.
#[tracing::instrument(skip(pool))]
pub async fn delete(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    force: bool,
) -> Result<(), DomainError> {
    let summary = db_containers::subtree_summary(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("container"))?;

    if summary.items > 0 {
        return Err(DomainError::NotEmpty {
            kind: "container",
            items: summary.items,
        });
    }
    if !force && (summary.lists > 0 || summary.containers > 0) {
        return Err(DomainError::HasChildren {
            kind: "container",
            lists: summary.lists,
            containers: summary.containers,
        });
    }

    match db_containers::delete_tree(pool, id, user_id, false).await? {
        DeleteTreeOutcome::Deleted => Ok(()),
        DeleteTreeOutcome::NotFound => Err(DomainError::NotFound("container")),
        DeleteTreeOutcome::NotEmpty { items } => Err(DomainError::NotEmpty {
            kind: "container",
            items,
        }),
    }
}

/// Archive or unarchive a container and all its descendants + their lists.
///
/// Idempotent: the whole subtree is set to the given state, so calling it twice
/// with the same `archived` leaves one state. `None` means the container does
/// not belong to `user_id`.
#[tracing::instrument(skip(pool), fields(container_id = %id, archived))]
pub async fn set_archived(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    archived: bool,
) -> Result<Option<Container>, DomainError> {
    if !db_containers::set_archived(pool, id, user_id, archived).await? {
        return Ok(None);
    }
    Ok(db_containers::get_one(pool, id, user_id)
        .await?
        .map(row_to_container))
}

/// Toggle archived state for a container and all its descendants + their lists.
#[tracing::instrument(skip(pool))]
pub async fn toggle_archive(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<Container>, DomainError> {
    let Some(current) = db_containers::get_one(pool, id, user_id).await? else {
        return Ok(None);
    };
    set_archived(pool, id, user_id, !current.archived).await
}

/// List archived containers (root entries only).
#[tracing::instrument(skip(pool))]
pub async fn list_archived(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<Container>, DomainError> {
    let rows = db_containers::list_archived(pool, user_id).await?;
    Ok(rows.into_iter().map(row_to_container).collect())
}

/// Move a container to a new parent/position, with cycle and hierarchy validation.
#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &MoveContainerRequest,
) -> Result<Container, DomainError> {
    // Phase 2: THINK — validate not moving to self
    rules::containers::validate_move(id, req.parent_container_id.as_deref())?;

    // Phase 1: READ — if a parent is specified, validate it
    if let Some(parent_id) = req.parent_container_id.as_deref() {
        let parent = db_containers::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;

        // Validate parent is a folder
        rules::containers::validate_hierarchy(parent.status.as_deref())?;

        // Validate no circular move (parent must not be a descendant of id)
        let circular = db_containers::is_descendant(pool, user_id, id, parent_id).await?;
        if circular {
            return Err(DomainError::Validation("circular_container_move"));
        }
    }

    // Phase 3: WRITE — compute position and move
    let position = match req.position {
        Some(p) => p,
        None => {
            db_containers::next_position(pool, user_id, req.parent_container_id.as_deref()).await?
        }
    };

    db_containers::move_container(
        pool,
        id,
        user_id,
        req.parent_container_id.as_deref(),
        position,
    )
    .await?
    .map(row_to_container)
    .ok_or(DomainError::NotFound("container"))
}

/// Rewrite the order of `parent_container_id`'s direct children, atomically.
///
/// Rules (same as [`move_container`]): the parent must belong to `user_id`, must
/// be a folder (`status IS NULL`), no container may become its own parent and no
/// cycle may be created. Every id must already live under `parent_container_id`
/// and belong to `user_id`, otherwise nothing is written.
#[tracing::instrument(skip(pool, ordered_ids), fields(parent = ?parent_container_id, count = ordered_ids.len()))]
pub async fn reorder(
    pool: &SqlitePool,
    user_id: &str,
    parent_container_id: Option<&str>,
    ordered_ids: &[String],
) -> Result<(), DomainError> {
    if let Some(parent_id) = parent_container_id {
        // Phase 1 READ: parent ownership
        let parent = db_containers::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
        // Phase 2 THINK: folder rule, self-parent, cycles
        rules::containers::validate_hierarchy(parent.status.as_deref())?;
        for id in ordered_ids {
            rules::containers::validate_move(id, Some(parent_id))?;
            if db_containers::is_descendant(pool, user_id, id, parent_id).await? {
                return Err(DomainError::Validation("circular_container_move"));
            }
        }
    }
    let scope = db_containers::child_ids(pool, user_id, parent_container_id).await?;
    DomainError::ensure_ids_in_scope(&scope, ordered_ids, "container")?;

    // Phase 3 WRITE: one transaction, all or nothing
    let mut tx = pool.begin().await.map_err(kartoteka_db::DbError::Sqlx)?;
    for (pos, id) in ordered_ids.iter().enumerate() {
        let moved = db_containers::move_container_in_tx(
            &mut tx,
            id,
            user_id,
            parent_container_id,
            pos as i32,
        )
        .await?;
        if moved.is_none() {
            return Err(DomainError::NotFound("container"));
        }
    }
    tx.commit().await.map_err(kartoteka_db::DbError::Sqlx)?;
    Ok(())
}

/// Pin or unpin a container to an explicit state.
///
/// Idempotent. `None` means the container does not belong to `user_id`.
#[tracing::instrument(skip(pool), fields(container_id = %id, pinned))]
pub async fn set_pinned(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    pinned: bool,
) -> Result<Option<Container>, DomainError> {
    Ok(db_containers::set_pinned(pool, id, user_id, pinned)
        .await?
        .map(row_to_container))
}

/// Toggle the pinned state of a container.
#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Container, DomainError> {
    let current = get_one(pool, id, user_id).await?;
    set_pinned(pool, id, user_id, !current.pinned)
        .await?
        .ok_or(DomainError::NotFound("container"))
}

/// Get progress statistics for a container.
///
/// Recursive: counts cover the container itself, every sub-container at any
/// depth, every list in that subtree and those lists' sublists at any depth.
#[tracing::instrument(skip(pool))]
pub async fn get_progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgress, DomainError> {
    // Ownership check
    get_one(pool, id, user_id).await?;
    let row = db_containers::progress(pool, id, user_id).await?;
    Ok(progress_row_to_domain(row))
}

/// Touch last_opened_at for a container.
#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    db_containers::touch_last_opened(pool, id, user_id).await?;
    Ok(())
}

/// Get direct children of a container (with parent ownership check).
#[tracing::instrument(skip(pool))]
pub async fn get_children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<Container>, DomainError> {
    // Ownership check on parent
    get_one(pool, parent_id, user_id).await?;
    let rows = db_containers::children(pool, parent_id, user_id).await?;
    Ok(rows.into_iter().map(row_to_container).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    async fn make_container(pool: &SqlitePool, user_id: &str, name: &str) -> Container {
        create(
            pool,
            user_id,
            &CreateContainerRequest {
                name: name.into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: None,
                color: None,
            },
        )
        .await
        .unwrap()
    }

    async fn make_child_container(
        pool: &SqlitePool,
        user_id: &str,
        name: &str,
        parent_container_id: &str,
    ) -> Container {
        create(
            pool,
            user_id,
            &CreateContainerRequest {
                name: name.into(),
                icon: None,
                color: None,
                description: None,
                status: None,
                parent_container_id: Some(parent_container_id.to_string()),
            },
        )
        .await
        .unwrap()
    }

    async fn make_list(
        pool: &SqlitePool,
        user_id: &str,
        name: &str,
        container_id: &str,
        parent_list_id: Option<&str>,
    ) -> String {
        crate::lists::create(
            pool,
            user_id,
            &crate::lists::CreateListRequest {
                name: name.into(),
                list_type: Some("checklist".into()),
                icon: None,
                color: None,
                description: None,
                container_id: Some(container_id.to_string()),
                parent_list_id: parent_list_id.map(str::to_string),
                features: vec![],
            },
        )
        .await
        .unwrap()
        .id
    }

    async fn make_item(pool: &SqlitePool, list_id: &str, title: &str) {
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, ?, 0)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(list_id)
            .bind(title)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_and_get_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let c = make_container(&pool, &uid, "My Container").await;
        assert_eq!(c.name, "My Container");
        assert_eq!(c.user_id, uid);
        assert!(!c.id.is_empty());

        let fetched = get_one(&pool, &c.id, &uid).await.unwrap();
        assert_eq!(fetched.id, c.id);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_not_found() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = get_one(&pool, "no-such-id", &uid).await;
        assert!(matches!(result, Err(DomainError::NotFound("container"))));
    }

    #[tokio::test]
    async fn create_under_folder_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // Folder has status = None
        let folder = make_container(&pool, &uid, "Folder").await;

        let child = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Child".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(folder.id.clone()),
                color: None,
            },
        )
        .await;

        assert!(child.is_ok());
        assert_eq!(
            child.unwrap().parent_container_id.as_deref(),
            Some(folder.id.as_str())
        );
    }

    #[tokio::test]
    async fn create_under_project_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // Project has status = Some("active")
        let project = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Project".into(),
                icon: None,
                description: None,
                status: Some("active".into()),
                parent_container_id: None,
                color: None,
            },
        )
        .await
        .unwrap();

        let result = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Child".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(project.id.clone()),
                color: None,
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(DomainError::Validation("invalid_container_hierarchy"))
        ));
    }

    #[tokio::test]
    async fn positions_auto_increment() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let a = make_container(&pool, &uid, "A").await;
        let b = make_container(&pool, &uid, "B").await;
        let c = make_container(&pool, &uid, "C").await;

        assert_eq!(a.position, 0);
        assert_eq!(b.position, 1);
        assert_eq!(c.position, 2);
    }

    #[tokio::test]
    async fn update_returns_patched_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let c = make_container(&pool, &uid, "Original").await;

        let updated = update(
            &pool,
            &c.id,
            &uid,
            &UpdateContainerRequest {
                name: Some("Updated".into()),
                icon: Some(Some("📁".into())),
                description: None,
                status: None,
                location_id: None,
                color: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.icon.as_deref(), Some("📁"));
    }

    #[tokio::test]
    async fn delete_removes_and_not_found_after() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let c = make_container(&pool, &uid, "ToDelete").await;
        delete(&pool, &c.id, &uid, false).await.unwrap();

        let result = get_one(&pool, &c.id, &uid).await;
        assert!(matches!(result, Err(DomainError::NotFound("container"))));
    }

    #[tokio::test]
    async fn move_to_self_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let c = make_container(&pool, &uid, "Solo").await;

        let result = move_container(
            &pool,
            &c.id,
            &uid,
            &MoveContainerRequest {
                parent_container_id: Some(c.id.clone()),
                position: None,
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(DomainError::Validation("cannot_move_to_self"))
        ));
    }

    #[tokio::test]
    async fn move_to_descendant_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let parent = make_container(&pool, &uid, "Parent").await;
        let child = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Child".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(parent.id.clone()),
                color: None,
            },
        )
        .await
        .unwrap();

        // Try to move parent under its own child (circular)
        let result = move_container(
            &pool,
            &parent.id,
            &uid,
            &MoveContainerRequest {
                parent_container_id: Some(child.id.clone()),
                position: None,
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(DomainError::Validation("circular_container_move"))
        ));
    }

    #[tokio::test]
    async fn toggle_pin_twice_restores_state() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let c = make_container(&pool, &uid, "Pinnable").await;
        assert!(!c.pinned);

        let after_first = toggle_pin(&pool, &c.id, &uid).await.unwrap();
        assert!(after_first.pinned);

        let after_second = toggle_pin(&pool, &c.id, &uid).await.unwrap();
        assert!(!after_second.pinned);
    }

    #[tokio::test]
    async fn list_all_returns_domain_types() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        make_container(&pool, &uid, "Alpha").await;
        make_container(&pool, &uid, "Beta").await;

        let all = list_all(&pool, &uid).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn toggle_archive_flips_and_returns_updated() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let c = make_container(&pool, &uid, "Archivable").await;
        assert!(!c.archived);

        let updated = toggle_archive(&pool, &c.id, &uid).await.unwrap().unwrap();
        assert!(updated.archived);

        let restored = toggle_archive(&pool, &c.id, &uid).await.unwrap().unwrap();
        assert!(!restored.archived);
    }

    #[tokio::test]
    async fn list_all_excludes_archived_containers() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let a = make_container(&pool, &uid, "A").await;
        make_container(&pool, &uid, "B").await;

        toggle_archive(&pool, &a.id, &uid).await.unwrap();

        let all = list_all(&pool, &uid).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "B");
    }

    #[tokio::test]
    async fn list_archived_returns_archived_containers() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let a = make_container(&pool, &uid, "A").await;
        make_container(&pool, &uid, "B").await;

        toggle_archive(&pool, &a.id, &uid).await.unwrap();

        let archived = list_archived(&pool, &uid).await.unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].name, "A");
    }

    // ── reorder (atomic, authorization-checked) ─────────────────────────────

    async fn make_child(pool: &SqlitePool, user_id: &str, name: &str, parent: &str) -> Container {
        create(
            pool,
            user_id,
            &CreateContainerRequest {
                name: name.into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(parent.into()),
                color: None,
            },
        )
        .await
        .unwrap()
    }

    async fn stored_parent_and_position(
        pool: &SqlitePool,
        user_id: &str,
        id: &str,
    ) -> (Option<String>, i32) {
        let row = db_containers::get_one(pool, id, user_id)
            .await
            .unwrap()
            .expect("container row");
        (row.parent_container_id, row.position)
    }

    #[tokio::test]
    async fn reorder_assigns_positions_in_given_order() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let a = make_child(&pool, &uid, "A", &parent.id).await;
        let b = make_child(&pool, &uid, "B", &parent.id).await;
        let c = make_child(&pool, &uid, "C", &parent.id).await;

        reorder(
            &pool,
            &uid,
            Some(&parent.id),
            &[c.id.clone(), a.id.clone(), b.id.clone()],
        )
        .await
        .unwrap();

        let ordered = db_containers::child_ids(&pool, &uid, Some(&parent.id))
            .await
            .unwrap();
        assert_eq!(ordered, vec![c.id.clone(), a.id.clone(), b.id.clone()]);
        for (expected, id) in [&c.id, &a.id, &b.id].iter().enumerate() {
            assert_eq!(
                stored_parent_and_position(&pool, &uid, id).await,
                (Some(parent.id.clone()), expected as i32)
            );
        }
    }

    #[tokio::test]
    async fn reorder_creating_cycle_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // a → b, so moving `a` under `b` would create a cycle.
        let a = make_container(&pool, &uid, "A").await;
        let b = make_child(&pool, &uid, "B", &a.id).await;

        let err = reorder(&pool, &uid, Some(&b.id), std::slice::from_ref(&a.id))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DomainError::Validation("circular_container_move")),
            "got {err:?}"
        );
        assert_eq!(
            stored_parent_and_position(&pool, &uid, &a.id).await,
            (None, 0)
        );
    }

    #[tokio::test]
    async fn reorder_making_container_its_own_parent_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let folder = make_container(&pool, &uid, "Folder").await;

        let err = reorder(
            &pool,
            &uid,
            Some(&folder.id),
            std::slice::from_ref(&folder.id),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::Validation("cannot_move_to_self")),
            "got {err:?}"
        );
        assert_eq!(
            stored_parent_and_position(&pool, &uid, &folder.id).await,
            (None, 0)
        );
    }

    #[tokio::test]
    async fn reorder_under_project_parent_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let project = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Project".into(),
                icon: None,
                description: None,
                status: Some("active".into()),
                parent_container_id: None,
                color: None,
            },
        )
        .await
        .unwrap();

        // A folder nested in a project is not creatable through the domain, so
        // write the bad state directly to prove `reorder` rejects it.
        let child_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO containers (id, user_id, name, parent_container_id, position) \
             VALUES (?, ?, 'Child', ?, 0)",
        )
        .bind(&child_id)
        .bind(&uid)
        .bind(&project.id)
        .execute(&pool)
        .await
        .unwrap();

        let err = reorder(
            &pool,
            &uid,
            Some(&project.id),
            std::slice::from_ref(&child_id),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::Validation("invalid_container_hierarchy")),
            "got {err:?}"
        );
        assert_eq!(
            stored_parent_and_position(&pool, &uid, &child_id).await,
            (Some(project.id.clone()), 0)
        );
    }

    #[tokio::test]
    async fn reorder_under_foreign_parent_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let foreign_folder = make_container(&pool, &intruder, "Foreign").await;
        let mine = make_container(&pool, &uid, "Mine").await;

        let err = reorder(
            &pool,
            &uid,
            Some(&foreign_folder.id),
            std::slice::from_ref(&mine.id),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::NotFound("container")),
            "got {err:?}"
        );
        assert_eq!(
            stored_parent_and_position(&pool, &uid, &mine.id).await,
            (None, 0)
        );
    }

    // ── Recursive progress ────────────────────────────────────────────────────

    #[tokio::test]
    async fn progress_counts_subcontainers_and_sublists() {
        use kartoteka_db::test_helpers::{
            create_test_item, create_test_list_in_container, create_test_sublist,
        };
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let root = make_container(&pool, &uid, "Root").await;
        let child = make_child(&pool, &uid, "Child", &root.id).await;

        // Root list with two items, one completed.
        let root_list = create_test_list_in_container(&pool, &uid, &root.id).await;
        let a = create_test_item(&pool, &root_list).await;
        create_test_item(&pool, &root_list).await;
        sqlx::query("UPDATE items SET completed = 1 WHERE id = ?")
            .bind(&a)
            .execute(&pool)
            .await
            .unwrap();

        // Child list with one item, plus a sublist with one completed item.
        let child_list = create_test_list_in_container(&pool, &uid, &child.id).await;
        create_test_item(&pool, &child_list).await;
        let sublist = create_test_sublist(&pool, &uid, &child_list).await;
        let done = create_test_item(&pool, &sublist).await;
        sqlx::query("UPDATE items SET completed = 1 WHERE id = ?")
            .bind(&done)
            .execute(&pool)
            .await
            .unwrap();

        let progress = get_progress(&pool, &root.id, &uid).await.unwrap();
        assert_eq!(progress.total_lists, 3);
        assert_eq!(progress.total_items, 4);
        assert_eq!(progress.completed_items, 2);

        // The child sees only its own subtree.
        let progress = get_progress(&pool, &child.id, &uid).await.unwrap();
        assert_eq!(progress.total_lists, 2);
        assert_eq!(progress.total_items, 2);
        assert_eq!(progress.completed_items, 1);
    }

    #[tokio::test]
    async fn progress_is_zero_for_empty_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let empty = make_container(&pool, &uid, "Empty").await;

        let progress = get_progress(&pool, &empty.id, &uid).await.unwrap();
        assert_eq!(progress.total_lists, 0);
        assert_eq!(progress.total_items, 0);
        assert_eq!(progress.completed_items, 0);
    }

    #[tokio::test]
    async fn progress_ignores_other_users_lists() {
        use kartoteka_db::test_helpers::{create_test_item, create_test_list_in_container};
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let root = make_container(&pool, &uid, "Root").await;

        let foreign_list = create_test_list_in_container(&pool, &other, &root.id).await;
        create_test_item(&pool, &foreign_list).await;

        let progress = get_progress(&pool, &root.id, &uid).await.unwrap();
        assert_eq!(progress.total_lists, 0);
        assert_eq!(progress.total_items, 0);
    }

    #[tokio::test]
    async fn reorder_with_foreign_id_writes_nothing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let a = make_child(&pool, &uid, "A", &parent.id).await;
        let b = make_child(&pool, &uid, "B", &parent.id).await;
        let foreign = make_container(&pool, &intruder, "Foreign").await;

        // `a` first: a partial write would already have changed its position.
        let err = reorder(
            &pool,
            &uid,
            Some(&parent.id),
            &[b.id.clone(), foreign.id.clone(), a.id.clone()],
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::NotFound("container")),
            "got {err:?}"
        );
        assert_eq!(
            stored_parent_and_position(&pool, &uid, &a.id).await,
            (Some(parent.id.clone()), 0)
        );
        assert_eq!(
            stored_parent_and_position(&pool, &uid, &b.id).await,
            (Some(parent.id), 1)
        );
    }

    #[tokio::test]
    async fn set_status_switches_folder_to_project_and_back() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = make_container(&pool, &uid, "Maybe project").await;
        assert!(container.status.is_none());

        let project = set_status(&pool, &container.id, &uid, Some("active"))
            .await
            .unwrap();
        assert_eq!(project.status.as_deref(), Some("active"));

        let folder = set_status(&pool, &container.id, &uid, None).await.unwrap();
        assert!(folder.status.is_none());
    }

    #[tokio::test]
    async fn set_status_rejects_unknown_value() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = make_container(&pool, &uid, "Box").await;

        let err = set_status(&pool, &container.id, &uid, Some("archived"))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("invalid_container_status")
        ));
    }

    #[tokio::test]
    async fn set_status_rejects_project_for_folder_with_children() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let child_req = CreateContainerRequest {
            name: "Child".into(),
            icon: None,
            color: None,
            description: None,
            status: None,
            parent_container_id: Some(parent.id.clone()),
        };
        create(&pool, &uid, &child_req).await.unwrap();

        let err = set_status(&pool, &parent.id, &uid, Some("active"))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("container_has_children")
        ));
        let unchanged = get_one(&pool, &parent.id, &uid).await.unwrap();
        assert!(unchanged.status.is_none());
    }

    #[tokio::test]
    async fn set_appearance_sets_and_clears_color() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = make_container(&pool, &uid, "Styled").await;

        let styled = set_appearance(
            &pool,
            &container.id,
            &uid,
            Some(Some("emoji:📦".into())),
            Some(Some("#00ff00".into())),
        )
        .await
        .unwrap();
        assert_eq!(styled.icon.as_deref(), Some("emoji:📦"));
        assert_eq!(styled.color.as_deref(), Some("#00ff00"));

        let cleared = set_appearance(&pool, &container.id, &uid, Some(None), Some(None))
            .await
            .unwrap();
        assert!(cleared.icon.is_none());
        assert!(cleared.color.is_none());
    }
    // ── delete matrix ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_empty_leaf_container_ok_without_force() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "Doomed").await;

        delete(&pool, &c.id, &uid, false).await.unwrap();

        assert!(get_one(&pool, &c.id, &uid).await.is_err());
    }

    #[tokio::test]
    async fn delete_container_with_descendants_requires_force() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let child = make_child_container(&pool, &uid, "Child", &parent.id).await;
        let list = make_list(&pool, &uid, "List", &child.id, None).await;

        let err = delete(&pool, &parent.id, &uid, false).await.unwrap_err();
        assert!(
            matches!(
                err,
                DomainError::HasChildren {
                    kind: "container",
                    lists: 1,
                    containers: 1
                }
            ),
            "got {err:?}"
        );
        assert!(get_one(&pool, &parent.id, &uid).await.is_ok());

        delete(&pool, &parent.id, &uid, true).await.unwrap();

        assert!(get_one(&pool, &parent.id, &uid).await.is_err());
        assert!(get_one(&pool, &child.id, &uid).await.is_err());
        assert!(
            crate::lists::get_one(&pool, &list, &uid)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn delete_container_with_item_in_nested_list_is_rejected_even_with_force() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let child = make_child_container(&pool, &uid, "Child", &parent.id).await;
        let list = make_list(&pool, &uid, "List", &child.id, None).await;
        let sublist = make_list(&pool, &uid, "Sublist", &child.id, Some(&list)).await;
        make_item(&pool, &sublist, "Deep item").await;

        let err = delete(&pool, &parent.id, &uid, true).await.unwrap_err();
        assert!(
            matches!(
                err,
                DomainError::NotEmpty {
                    kind: "container",
                    items: 1
                }
            ),
            "got {err:?}"
        );
        assert!(get_one(&pool, &parent.id, &uid).await.is_ok());
        assert!(get_one(&pool, &child.id, &uid).await.is_ok());
    }

    #[tokio::test]
    async fn delete_container_of_other_user_is_not_found() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let c = make_container(&pool, &owner, "Mine").await;

        let err = delete(&pool, &c.id, &intruder, true).await.unwrap_err();
        assert!(
            matches!(err, DomainError::NotFound("container")),
            "got {err:?}"
        );
        assert!(get_one(&pool, &c.id, &owner).await.is_ok());
    }

    #[tokio::test]
    async fn delete_preview_reports_container_tree() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let child = make_child_container(&pool, &uid, "Child", &parent.id).await;
        let list = make_list(&pool, &uid, "List", &child.id, None).await;
        make_item(&pool, &list, "Item").await;

        let preview = delete_preview(&pool, &parent.id, &uid).await.unwrap();
        assert_eq!(
            preview,
            kartoteka_shared::DeletePreview {
                lists: 1,
                containers: 1,
                items: 1
            }
        );

        let intruder = create_test_user(&pool).await;
        assert!(matches!(
            delete_preview(&pool, &parent.id, &intruder)
                .await
                .unwrap_err(),
            DomainError::NotFound("container")
        ));
    }

    #[tokio::test]
    async fn set_archived_is_idempotent_and_cascades() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let child = make_child_container(&pool, &uid, "Child", &parent.id).await;
        let list = make_list(&pool, &uid, "List", &child.id, None).await;

        for _ in 0..2 {
            let archived = set_archived(&pool, &parent.id, &uid, true)
                .await
                .unwrap()
                .unwrap();
            assert!(archived.archived);
        }

        let child_after = get_one(&pool, &child.id, &uid).await.unwrap();
        assert!(child_after.archived, "the sub-container is archived");
        let list_row: (i64,) = sqlx::query_as("SELECT archived FROM lists WHERE id = ?")
            .bind(&list)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(list_row.0, 1, "the list of the sub-container is archived");

        // unarchiving twice restores the whole subtree
        for _ in 0..2 {
            let restored = set_archived(&pool, &parent.id, &uid, false)
                .await
                .unwrap()
                .unwrap();
            assert!(!restored.archived);
        }
        let child_after = get_one(&pool, &child.id, &uid).await.unwrap();
        assert!(!child_after.archived);
        let list_row: (i64,) = sqlx::query_as("SELECT archived FROM lists WHERE id = ?")
            .bind(&list)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(list_row.0, 0);
    }

    #[tokio::test]
    async fn set_pinned_is_idempotent() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "Pinnable").await;

        for _ in 0..2 {
            let pinned = set_pinned(&pool, &c.id, &uid, true).await.unwrap().unwrap();
            assert!(pinned.pinned);
        }
        let unpinned = set_pinned(&pool, &c.id, &uid, false)
            .await
            .unwrap()
            .unwrap();
        assert!(!unpinned.pinned);
    }

    #[tokio::test]
    async fn set_archived_and_set_pinned_of_other_user_are_none() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "Mine").await;

        assert!(
            set_archived(&pool, &c.id, &intruder, true)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            set_pinned(&pool, &c.id, &intruder, true)
                .await
                .unwrap()
                .is_none()
        );
        let row = get_one(&pool, &c.id, &uid).await.unwrap();
        assert!(!row.archived);
        assert!(!row.pinned);
    }
}
