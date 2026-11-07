use http::request::Parts;
use kartoteka_domain as domain;
use rmcp::{
    ErrorData,
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::server::KartotekaServer;

#[derive(Deserialize, JsonSchema)]
pub struct AddRelationParams {
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    /// "blocks" or "relates_to"
    pub relation_type: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct RemoveRelationParams {
    pub relation_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetRelationsParams {
    /// "item", "list" or "container"
    pub entity_type: String,
    pub entity_id: String,
}

#[tool_router(router = relations_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "add_relation", description = "mcp-tool-add_relation-desc")]
    pub(crate) async fn add_relation(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AddRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let rel = domain::relations::create(
            &self.pool,
            &uid,
            &p.from_type,
            &p.from_id,
            &p.to_type,
            &p.to_id,
            &p.relation_type,
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(rel, &locale)
    }

    #[rmcp::tool(
        name = "remove_relation",
        description = "mcp-tool-remove_relation-desc"
    )]
    pub(crate) async fn remove_relation(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<RemoveRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        domain::relations::delete(&self.pool, &uid, &p.relation_id)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(json!({"deleted": true}), &locale)
    }

    #[rmcp::tool(name = "get_relations", description = "mcp-tool-get_relations-desc")]
    pub(crate) async fn get_relations(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetRelationsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let rels =
            domain::relations::get_for_entity(&self.pool, &uid, &p.entity_type, &p.entity_id)
                .await
                .map_err(self.domain_err(&locale))?;
        self.json_result(rels, &locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::*;
    use kartoteka_db::test_helpers::{create_test_item, create_test_user};

    /// Two items in the owner's list, related with `blocks`.
    async fn related_items(pool: &sqlx::SqlitePool, uid: &str) -> (String, String) {
        let list_id = insert_test_list(pool, uid).await;
        let item_a = create_test_item(pool, &list_id).await;
        let item_b = create_test_item(pool, &list_id).await;
        let server = test_server(pool.clone());
        server
            .add_relation(
                Extension(parts_for_user(uid)),
                Parameters(AddRelationParams {
                    from_type: "item".to_owned(),
                    from_id: item_a.clone(),
                    to_type: "item".to_owned(),
                    to_id: item_b,
                    relation_type: "blocks".to_owned(),
                }),
            )
            .await
            .unwrap();
        (list_id, item_a)
    }

    #[tokio::test]
    async fn get_relations_returns_relations_of_an_owned_item() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let (_list_id, item_a) = related_items(&pool, &uid).await;
        let server = test_server(pool);

        let result = server
            .get_relations(
                Extension(parts_for_user(&uid)),
                Parameters(GetRelationsParams {
                    entity_type: "item".to_owned(),
                    entity_id: item_a,
                }),
            )
            .await
            .unwrap();

        let data = item_json(&result).await;
        let relations = data.as_array().expect("a list of relations");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0]["relation_type"], "blocks");
        assert_eq!(relations[0]["other_type"], "item");
        assert_eq!(relations[0]["other_title"], "Test Item");
    }

    #[tokio::test]
    async fn get_relations_for_a_foreign_entity_is_forbidden() {
        let pool = multi_conn_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let (list_id, _item_a) = related_items(&pool, &owner).await;
        let server = test_server(pool);

        let err = server
            .get_relations(
                Extension(parts_for_user(&attacker)),
                Parameters(GetRelationsParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id,
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32600);
        assert_eq!(err.data.as_ref().unwrap()["error"], "forbidden");
    }

    #[tokio::test]
    async fn get_relations_with_unknown_entity_type_is_a_validation_error() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);

        let err = server
            .get_relations(
                Extension(parts_for_user(&uid)),
                Parameters(GetRelationsParams {
                    entity_type: "spaceship".to_owned(),
                    entity_id: "any".to_owned(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602);
        let data = err.data.as_ref().unwrap();
        assert_eq!(data["error"], "validation");
        assert_eq!(data["reason"], "invalid_entity_type");
    }
}
