use crate::{DomainError, rules};
use kartoteka_db::types::ContainerRow;
use kartoteka_db::{SqlitePool, containers as db_containers};
use kartoteka_shared::types::{
    Container, ContainerProgress, CreateContainerRequest, MoveContainerRequest,
    UpdateContainerRequest,
};

fn row_to_container(r: ContainerRow) -> Container {
    Container {
        id: r.id,
        user_id: r.user_id,
        name: r.name,
        icon: r.icon,
        description: r.description,
        status: r.status,
        parent_container_id: r.parent_container_id,
        position: r.position,
        pinned: r.pinned,
        last_opened_at: r.last_opened_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
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

/// Create a new container, validating hierarchy rules.
#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
) -> Result<Container, DomainError> {
    rules::containers::validate_status(req.status.as_deref())?;

    // Phase 1: READ — if a parent is specified, fetch it
    if let Some(parent_id) = req.parent_container_id.as_deref() {
        let parent = db_containers::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
        // Phase 2: THINK — validate that parent is a folder
        rules::containers::validate_hierarchy(parent.status.as_deref())?;
    }

    // Phase 3: WRITE — compute position, then insert
    let position =
        db_containers::next_position(pool, user_id, req.parent_container_id.as_deref()).await?;
    let row = db_containers::insert(pool, user_id, req, position).await?;
    Ok(row_to_container(row))
}

/// Update a container's metadata.
#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Container, DomainError> {
    db_containers::update(pool, id, user_id, req)
        .await?
        .map(row_to_container)
        .ok_or(DomainError::NotFound("container"))
}

/// Delete a container.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), DomainError> {
    let deleted = db_containers::delete(pool, id, user_id).await?;
    if !deleted {
        return Err(DomainError::NotFound("container"));
    }
    Ok(())
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

/// Toggle the pinned state of a container.
#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Container, DomainError> {
    db_containers::toggle_pin(pool, id, user_id)
        .await?
        .map(row_to_container)
        .ok_or(DomainError::NotFound("container"))
}

/// Get progress statistics for a container.
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
            },
        )
        .await
        .unwrap()
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
        delete(&pool, &c.id, &uid).await.unwrap();

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
}
