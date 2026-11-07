//! The `/trash` queue: MCP agents mark entities instead of deleting them (#226).
//!
//! `toggle_mark_for_delete` is the only way an agent can get rid of something, and
//! it deletes nothing: the row lands in the user's pending-deletions queue, the
//! user reviews it on `/trash` and decides. `list_marked_for_delete` lets an agent
//! see what it has already queued so it does not toggle an entity off by accident.

use http::request::Parts;
use kartoteka_domain as domain;
use kartoteka_shared::DeletableEntity;
use rmcp::{
    ErrorData,
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::McpError;
use crate::server::KartotekaServer;

#[derive(Deserialize, JsonSchema)]
pub struct ToggleMarkForDeleteParams {
    /// "item", "list", "container" or "comment"
    pub entity_type: String,
    pub entity_id: String,
}

#[tool_router(router = pending_deletions_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(
        name = "toggle_mark_for_delete",
        description = "mcp-tool-toggle_mark_for_delete-desc"
    )]
    pub(crate) async fn toggle_mark_for_delete(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ToggleMarkForDeleteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let kind: DeletableEntity = p.entity_type.parse().map_err(|_| {
            self.map_err(
                McpError::BadRequest("invalid_entity_type".to_string()),
                &locale,
            )
        })?;
        let marked = domain::pending_deletions::toggle(&self.pool, &uid, kind, &p.entity_id)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(
            json!({
                "entity_type": kind,
                "entity_id": p.entity_id,
                "marked": marked,
            }),
            &locale,
        )
    }

    #[rmcp::tool(
        name = "list_marked_for_delete",
        description = "mcp-tool-list_marked_for_delete-desc"
    )]
    pub(crate) async fn list_marked_for_delete(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let items = domain::pending_deletions::list_all(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(items, &locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::{
        domain_container_req, insert_test_list, multi_conn_pool, parts_for_user, result_text,
        test_server,
    };
    use kartoteka_db::test_helpers::create_test_user;
    use serde_json::Value;

    /// One entity of every kind the queue accepts, all owned by `uid`.
    struct Owned {
        item: String,
        list: String,
        container: String,
        comment: String,
    }

    async fn make_owned(server: &KartotekaServer, uid: &str) -> Owned {
        let list = insert_test_list(&server.pool, uid).await;

        let item = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Passport')")
            .bind(&item)
            .bind(&list)
            .execute(&server.pool)
            .await
            .unwrap();

        let container =
            domain::containers::create(&server.pool, uid, &domain_container_req("Crate"))
                .await
                .unwrap()
                .id;

        let comment = domain::comments::create(
            &server.pool,
            uid,
            "item",
            &item,
            "don't forget",
            "assistant",
            None,
        )
        .await
        .unwrap()
        .id;

        Owned {
            item,
            list,
            container,
            comment,
        }
    }

    fn key(owned: &Owned, kind: DeletableEntity) -> &str {
        match kind {
            DeletableEntity::Item => &owned.item,
            DeletableEntity::List => &owned.list,
            DeletableEntity::Container => &owned.container,
            DeletableEntity::Comment => &owned.comment,
        }
    }

    async fn toggle(
        server: &KartotekaServer,
        uid: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Value, ErrorData> {
        let result = server
            .toggle_mark_for_delete(
                Extension(parts_for_user(uid)),
                Parameters(ToggleMarkForDeleteParams {
                    entity_type: entity_type.to_owned(),
                    entity_id: entity_id.to_owned(),
                }),
            )
            .await?;
        Ok(serde_json::from_str(&result_text(&result)).unwrap())
    }

    async fn queued(server: &KartotekaServer, uid: &str) -> Vec<Value> {
        let result = server
            .list_marked_for_delete(Extension(parts_for_user(uid)))
            .await
            .unwrap();
        serde_json::from_str(&result_text(&result)).unwrap()
    }

    #[tokio::test]
    async fn toggle_marks_then_unmarks_every_entity_type() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let owned = make_owned(&server, &uid).await;

        for kind in DeletableEntity::ALL {
            let id = key(&owned, kind);
            let marked = toggle(&server, &uid, kind.as_str(), id).await.unwrap();
            assert_eq!(marked["entity_type"], kind.as_str());
            assert_eq!(marked["entity_id"], id);
            assert_eq!(marked["marked"], true, "{kind} must be marked");

            let unmarked = toggle(&server, &uid, kind.as_str(), id).await.unwrap();
            assert_eq!(
                unmarked["marked"], false,
                "a second call must unmark {kind}"
            );
        }

        // Nothing was deleted by any of the calls above.
        for (table, id) in [
            ("items", &owned.item),
            ("lists", &owned.list),
            ("containers", &owned.container),
            ("comments", &owned.comment),
        ] {
            let row: Option<(i64,)> =
                sqlx::query_as(&format!("SELECT 1 FROM {table} WHERE id = ?"))
                    .bind(id)
                    .fetch_optional(&server.pool)
                    .await
                    .unwrap();
            assert!(row.is_some(), "{table} row {id} must survive marking");
        }
    }

    #[tokio::test]
    async fn toggle_foreign_entity_is_not_found() {
        let pool = multi_conn_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let server = test_server(pool);
        let owned = make_owned(&server, &owner).await;

        for kind in DeletableEntity::ALL {
            let err = toggle(&server, &intruder, kind.as_str(), key(&owned, kind))
                .await
                .unwrap_err();
            assert_eq!(err.code.0, -32602, "unexpected code for {kind}: {err:?}");
            let data = err.data.as_ref().expect("not_found carries data");
            assert_eq!(data["error"], "not_found", "{kind}: {data}");
            assert_eq!(data["entity"], kind.as_str(), "{kind}: {data}");
        }

        let queued: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pending_deletions")
            .fetch_one(&server.pool)
            .await
            .unwrap();
        assert_eq!(queued.0, 0, "a foreign mark must never write a row");
    }

    #[tokio::test]
    async fn toggle_with_unknown_entity_type_is_a_validation_error() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let owned = make_owned(&server, &uid).await;

        for locale in ["en", "pl"] {
            let err = server
                .toggle_mark_for_delete(
                    Extension(crate::server::test_support::parts_for_user_locale(
                        &uid, locale,
                    )),
                    Parameters(ToggleMarkForDeleteParams {
                        entity_type: "tag".to_owned(),
                        entity_id: owned.list.clone(),
                    }),
                )
                .await
                .unwrap_err();
            assert_eq!(err.code.0, -32602, "[{locale}] unexpected code: {err:?}");
            let data = err.data.as_ref().expect("validation carries data");
            assert_eq!(data["error"], "validation", "[{locale}] {data}");
            assert_eq!(data["reason"], "invalid_entity_type", "[{locale}] {data}");
        }
    }

    #[tokio::test]
    async fn list_returns_labels_context_and_only_the_users_own_rows() {
        let pool = multi_conn_pool().await;
        let owner = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let server = test_server(pool);
        let owned = make_owned(&server, &owner).await;
        let foreign = make_owned(&server, &other).await;

        // The owner queues an item, its list and a comment; the other user queues
        // their own list.
        for (kind, id) in [
            (DeletableEntity::Item, owned.item.as_str()),
            (DeletableEntity::List, owned.list.as_str()),
            (DeletableEntity::Comment, owned.comment.as_str()),
        ] {
            toggle(&server, &owner, kind.as_str(), id).await.unwrap();
        }
        toggle(&server, &other, "list", &foreign.list)
            .await
            .unwrap();

        let items = queued(&server, &owner).await;
        assert_eq!(items.len(), 3, "only the owner's rows: {items:?}");

        let find = |kind: &str| {
            items
                .iter()
                .find(|item| item["entity_type"] == kind)
                .unwrap_or_else(|| panic!("missing {kind} row"))
        };

        let item = find("item");
        assert_eq!(item["label"], "Passport");
        assert_eq!(item["context"], "Test List");
        assert_eq!(
            item["href"],
            format!("/lists/{}/items/{}", owned.list, owned.item)
        );
        assert!(item["marked_at"].as_str().is_some_and(|s| !s.is_empty()));

        let list = find("list");
        assert_eq!(list["label"], "Test List");
        assert_eq!(list["entity_id"], owned.list);

        let comment = find("comment");
        assert_eq!(comment["label"], "don't forget");
        assert_eq!(comment["context"], "Passport");

        assert!(
            items.iter().all(|item| item["entity_id"] != foreign.list),
            "another user's rows must not leak: {items:?}"
        );
    }

    #[tokio::test]
    async fn list_is_empty_before_anything_is_marked() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);

        let result = server
            .list_marked_for_delete(Extension(parts_for_user(&uid)))
            .await
            .unwrap();
        let items: Vec<Value> = serde_json::from_str(&result_text(&result)).unwrap();
        assert!(items.is_empty(), "unexpected rows: {items:?}");
    }
}
