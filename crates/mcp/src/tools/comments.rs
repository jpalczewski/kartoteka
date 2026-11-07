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

use crate::server::KartotekaServer;

#[derive(Deserialize, JsonSchema)]
pub struct ListCommentsParams {
    /// "item", "list" or "container"
    pub entity_type: String,
    pub entity_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AddCommentParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    /// Display name shown as comment author (e.g. "Claude"). Omit to attribute to the logged-in user.
    pub author_name: Option<String>,
}

#[tool_router(router = comments_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "add_comment", description = "mcp-tool-add_comment-desc")]
    pub(crate) async fn add_comment(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AddCommentParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let comment = domain::comments::create(
            &self.pool,
            &uid,
            &p.entity_type,
            &p.entity_id,
            &p.content,
            // Comments written through MCP come from an AI assistant, not from
            // the user typing: the UI renders `assistant` with an "AI" badge and
            // falls back to the author name. `author_name` is kept as given.
            "assistant",
            p.author_name.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(comment, &locale)
    }

    #[rmcp::tool(name = "list_comments", description = "mcp-tool-list_comments-desc")]
    pub(crate) async fn list_comments(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ListCommentsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let comments =
            domain::comments::list_for_entity(&self.pool, &uid, &p.entity_type, &p.entity_id)
                .await
                .map_err(self.domain_err(&locale))?;
        self.json_result(comments, &locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::*;
    use kartoteka_db::test_helpers::create_test_user;

    #[tokio::test]
    async fn list_comments_returns_comments_of_an_owned_list() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);

        server
            .add_comment(
                Extension(parts_for_user(&uid)),
                Parameters(AddCommentParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id.clone(),
                    content: "Looks good".to_owned(),
                    author_name: Some("Claude".to_owned()),
                }),
            )
            .await
            .unwrap();

        let result = server
            .list_comments(
                Extension(parts_for_user(&uid)),
                Parameters(ListCommentsParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id,
                }),
            )
            .await
            .unwrap();

        let data = item_json(&result).await;
        let comments = data.as_array().expect("a list of comments");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0]["content"], "Looks good");
        assert_eq!(comments[0]["author_type"], "assistant");
    }

    #[tokio::test]
    async fn list_comments_for_foreign_list_is_forbidden() {
        let pool = multi_conn_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &owner).await;
        let server = test_server(pool);

        let err = server
            .list_comments(
                Extension(parts_for_user(&attacker)),
                Parameters(ListCommentsParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id,
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32600, "forbidden is an invalid request");
        assert_eq!(err.data.as_ref().unwrap()["error"], "forbidden");
    }

    #[tokio::test]
    async fn list_comments_with_unknown_entity_type_is_a_validation_error() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);

        let err = server
            .list_comments(
                Extension(parts_for_user(&uid)),
                Parameters(ListCommentsParams {
                    entity_type: "spaceship".to_owned(),
                    entity_id: "any".to_owned(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "validation is invalid params");
        let data = err.data.as_ref().unwrap();
        assert_eq!(data["error"], "validation");
        assert_eq!(data["reason"], "invalid_entity_type");
    }
}
