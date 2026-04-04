mod children;
mod crud;
mod home;
mod pin;
mod reorder;

pub use children::get_children;
pub use crud::{create, delete, get_one, list_all, update};
pub use home::home;
pub use pin::toggle_pin;
pub use reorder::{move_container, reorder};

pub(super) const CONTAINER_SELECT: &str = "\
    SELECT c.id, c.user_id, c.name, c.description, c.status, \
    c.parent_container_id, c.position, c.pinned, c.last_opened_at, \
    c.created_at, c.updated_at \
    FROM containers c";

pub(super) async fn fetch_container_ids_in_scope(
    d1: &worker::D1Database,
    user_id: &str,
    parent_container_id: Option<&str>,
) -> worker::Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct ContainerIdRow {
        id: String,
    }

    let result = match parent_container_id {
        Some(parent_id) => {
            d1.prepare(
                "SELECT id FROM containers \
                 WHERE user_id = ?1 AND parent_container_id = ?2 \
                 ORDER BY position ASC, created_at ASC",
            )
            .bind(&[user_id.into(), parent_id.into()])?
            .all()
            .await?
        }
        None => {
            d1.prepare(
                "SELECT id FROM containers \
                 WHERE user_id = ?1 AND parent_container_id IS NULL \
                 ORDER BY position ASC, created_at ASC",
            )
            .bind(&[user_id.into()])?
            .all()
            .await?
        }
    };

    Ok(result
        .results::<ContainerIdRow>()?
        .into_iter()
        .map(|row| row.id)
        .collect())
}
