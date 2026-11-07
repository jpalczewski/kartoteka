use http::request::Parts;
use kartoteka_db as db;
use kartoteka_domain as domain;
use kartoteka_shared::auth_ctx::{UserId, UserLocale, UserTimezone};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext},
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
        ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, ResourceContents,
        ServerCapabilities, ServerConfig,
    },
    service::RequestContext,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

mod annotations;
pub(crate) mod batch;

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;

use crate::McpError;
use crate::McpI18n;

pub struct KartotekaServer {
    pub(crate) pool: SqlitePool,
    pub(crate) i18n: Arc<McpI18n>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl Clone for KartotekaServer {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            i18n: self.i18n.clone(),
            tool_router: Self::combined_router(),
        }
    }
}

impl KartotekaServer {
    pub fn new(pool: SqlitePool, i18n: Arc<McpI18n>) -> Self {
        Self {
            tool_router: Self::combined_router(),
            pool,
            i18n,
        }
    }

    /// The whole tool surface, merged from the per-area routers in
    /// `crate::tools`. Every area file owns one `#[tool_router(router = ...)]`
    /// block; this is the only place they are combined.
    fn combined_router() -> ToolRouter<Self> {
        Self::items_router()
            + Self::lists_router()
            + Self::lists_order_router()
            + Self::containers_router()
            + Self::comments_router()
            + Self::relations_router()
            + Self::time_router()
            + Self::tags_router()
            + Self::templates_router()
            + Self::search_router()
            + Self::pending_deletions_router()
    }

    /// Map any tool failure to a JSON-RPC error for the client.
    ///
    /// Every branch also puts a machine-readable code into `data`, so clients can
    /// react without parsing the localized message:
    /// `{"error": "already_exists", "kind": "list", "id": "..."}`.
    /// Internal causes (domain `Internal`, `Db`, serde) are logged on the server
    /// and never sent to the client.
    pub(crate) fn map_err(&self, e: McpError, locale: &str) -> ErrorData {
        /// Which JSON-RPC constructor the branch ends up in.
        enum Code {
            InvalidRequest,
            InvalidParams,
            Internal,
        }

        // Internal causes stay on the server; the client only sees `mcp-err-internal`.
        if let McpError::Domain(domain::DomainError::Internal(_) | domain::DomainError::Db(_))
        | McpError::Serde(_) = &e
        {
            tracing::error!(error = %e, "mcp tool call failed with an internal error");
        }

        let (code, key, args, data): (
            Code,
            &'static str,
            Vec<(&'static str, String)>,
            Option<serde_json::Value>,
        ) = match &e {
            McpError::Unauthorized => (
                Code::InvalidRequest,
                "mcp-err-unauthorized",
                vec![],
                Some(json!({"error": "unauthorized"})),
            ),
            McpError::Domain(domain::DomainError::NotFound(k)) => (
                Code::InvalidParams,
                "mcp-err-not-found",
                vec![("entity", (*k).to_owned())],
                Some(json!({"error": "not_found", "entity": k})),
            ),
            McpError::Domain(domain::DomainError::Validation(r)) => (
                Code::InvalidParams,
                "mcp-err-validation",
                vec![("reason", (*r).to_owned())],
                Some(json!({"error": "validation", "reason": r})),
            ),
            McpError::Domain(domain::DomainError::FeatureRequired(f)) => (
                Code::InvalidParams,
                "mcp-err-feature-required",
                vec![("feature", (*f).to_owned())],
                Some(json!({"error": "feature_required", "feature": f})),
            ),
            // `id` is the entity that already owns the name, so the client can
            // reuse it instead of retrying the same failing call (#374).
            McpError::Domain(domain::DomainError::AlreadyExists { kind, id }) => (
                Code::InvalidParams,
                "mcp-err-already-exists",
                vec![("kind", (*kind).to_owned()), ("id", id.clone())],
                Some(json!({"error": "already_exists", "kind": kind, "id": id})),
            ),
            McpError::Domain(domain::DomainError::NotEmpty { kind, items }) => (
                Code::InvalidRequest,
                "mcp-err-not-empty",
                vec![("kind", (*kind).to_owned()), ("items", items.to_string())],
                Some(json!({"error": "not_empty", "kind": kind, "items": items})),
            ),
            McpError::Domain(domain::DomainError::HasChildren {
                kind,
                lists,
                containers,
            }) => (
                Code::InvalidRequest,
                "mcp-err-has-children",
                vec![
                    ("kind", (*kind).to_owned()),
                    ("lists", lists.to_string()),
                    ("containers", containers.to_string()),
                ],
                Some(json!({
                    "error": "has_children",
                    "kind": kind,
                    "lists": lists,
                    "containers": containers,
                })),
            ),
            McpError::Domain(domain::DomainError::Forbidden) => (
                Code::InvalidRequest,
                "mcp-err-forbidden",
                vec![],
                Some(json!({"error": "forbidden"})),
            ),
            McpError::BadRequest(r) => (
                Code::InvalidParams,
                "mcp-err-validation",
                vec![("reason", r.clone())],
                Some(json!({"error": "validation", "reason": r})),
            ),
            McpError::Domain(domain::DomainError::Internal(_))
            | McpError::Domain(domain::DomainError::Db(_))
            | McpError::Serde(_) => (Code::Internal, "mcp-err-internal", vec![], None),
        };

        let arg_refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let msg = self.i18n.translate_args(locale, key, &arg_refs);
        match code {
            Code::InvalidRequest => ErrorData::invalid_request(msg, data),
            Code::InvalidParams => ErrorData::invalid_params(msg, data),
            Code::Internal => ErrorData::internal_error(msg, data),
        }
    }

    fn extract_user_id_and_locale(parts: &Parts) -> Result<(String, String), McpError> {
        let user_id = parts
            .extensions
            .get::<UserId>()
            .map(|u| u.0.clone())
            .ok_or(McpError::Unauthorized)?;
        let locale = parts
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        Ok((user_id, locale))
    }

    /// Pull `(user_id, locale)` out of request extensions, mapping any failure
    /// straight to a localized `ErrorData` so handlers can `?` it directly.
    pub(crate) fn auth(&self, parts: &Parts) -> Result<(String, String), ErrorData> {
        Self::extract_user_id_and_locale(parts).map_err(|e| self.map_err(e, "en"))
    }

    pub(crate) fn timezone(parts: &Parts) -> String {
        parts
            .extensions
            .get::<UserTimezone>()
            .map(|t| t.0.clone())
            .unwrap_or_else(|| "UTC".to_string())
    }

    /// Closure factory that maps a `DomainError` to localized `ErrorData`.
    /// Pass directly to `.map_err(...)` to avoid the noisy
    /// `|e| self.map_err(McpError::Domain(e), &locale)` boilerplate.
    pub(crate) fn domain_err<'a>(
        &'a self,
        locale: &'a str,
    ) -> impl Fn(domain::DomainError) -> ErrorData + 'a {
        move |e| self.map_err(McpError::Domain(e), locale)
    }

    /// Same idea for `db::DbError` — wraps via `Into<DomainError>`.
    pub(crate) fn db_err<'a>(&'a self, locale: &'a str) -> impl Fn(db::DbError) -> ErrorData + 'a {
        move |e| self.map_err(McpError::Domain(e.into()), locale)
    }

    /// Same idea for sqlx errors that occur outside `db::*` (begin/commit).
    pub(crate) fn sqlx_err<'a>(
        &'a self,
        locale: &'a str,
    ) -> impl Fn(sqlx::Error) -> ErrorData + 'a {
        move |e| self.map_err(McpError::Domain(db::DbError::Sqlx(e).into()), locale)
    }

    /// Like `domain_err` but intercepts `AlreadyExists` and returns
    /// `{"id": "<existing_id>", "existed": true}` instead of an error.
    pub(crate) fn domain_or_existing<T: serde::Serialize>(
        &self,
        result: Result<T, domain::DomainError>,
        locale: &str,
    ) -> Result<CallToolResult, ErrorData> {
        match result {
            Ok(v) => self.json_result(v, locale),
            Err(domain::DomainError::AlreadyExists { id, .. }) => {
                self.json_result(serde_json::json!({"id": id, "existed": true}), locale)
            }
            Err(e) => Err(self.domain_err(locale)(e)),
        }
    }

    pub(crate) fn json_result<T: serde::Serialize>(
        &self,
        value: T,
        locale: &str,
    ) -> Result<CallToolResult, ErrorData> {
        let v =
            serde_json::to_value(value).map_err(|e| self.map_err(McpError::Serde(e), locale))?;
        Ok(CallToolResult::success(vec![
            ContentBlock::json(v).map_err(|e| ErrorData::invalid_request(e.to_string(), None))?,
        ]))
    }

    /// Verify every id in `ids` belongs to `uid`. `kind_label` appears in the
    /// error message so callers can distinguish container vs parent-container
    /// references.
    pub(crate) async fn ensure_containers_owned(
        &self,
        uid: &str,
        ids: &[&str],
        kind_label: &str,
        locale: &str,
    ) -> Result<(), ErrorData> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut unique: Vec<&str> = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let owned = db::containers::find_owned_ids(&self.pool, uid, &unique)
            .await
            .map_err(self.db_err(locale))?;
        if unique.iter().any(|id| !owned.contains(*id)) {
            return Err(self.map_err(
                McpError::BadRequest(format!("one or more {kind_label} values not found")),
                locale,
            ));
        }
        Ok(())
    }

    /// Verify every id in `ids` belongs to `uid`. `kind_label` appears in the
    /// error message so callers can distinguish list references.
    pub(crate) async fn ensure_lists_owned(
        &self,
        uid: &str,
        ids: &[&str],
        kind_label: &str,
        locale: &str,
    ) -> Result<(), ErrorData> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut unique: Vec<&str> = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let owned = db::lists::find_owned_ids(&self.pool, uid, &unique)
            .await
            .map_err(self.db_err(locale))?;
        if unique.iter().any(|id| !owned.contains(*id)) {
            return Err(self.map_err(
                McpError::BadRequest(format!("one or more {kind_label} values not found")),
                locale,
            ));
        }
        Ok(())
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

impl ServerHandler for KartotekaServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "kartoteka",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Kartoteka is a personal task and list manager. \
            Start every session by calling list_lists to orient yourself — \
            you need list IDs for most item operations. \
            Use search_items to find items by keyword before acting on them. \
            When adding comments: omit author_name to write in the user's voice; \
            set author_name to your name (e.g. \"Claude\") for your own observations. \
            Prefer get_today and list_overdue to surface what needs attention now. \
            Deleting over MCP goes through toggle_mark_for_delete: it deletes nothing, \
            the user reviews the marked entities on /trash and decides, and \
            list_marked_for_delete shows what you already queued.",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        let mut tools = self.tool_router.list_all();
        for t in &mut tools {
            if let Some(key) = t.description.as_deref() {
                let translated = self.i18n.translate(&locale, key);
                t.description = Some(translated.into());
            }
            t.annotations = Some(annotations::for_tool(t.name.as_ref()));
        }
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        Ok(ListResourcesResult::with_all_items(
            crate::resources::static_resources(&self.i18n, &locale),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        Ok(ListResourceTemplatesResult::with_all_items(
            crate::resources::resource_templates(&self.i18n, &locale),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        use crate::resources::{ResourceUri, parse as parse_uri};

        let user_id = context
            .extensions
            .get::<UserId>()
            .map(|u| u.0.clone())
            .ok_or_else(|| ErrorData::invalid_request("unauthorized", None))?;
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        let timezone = context
            .extensions
            .get::<UserTimezone>()
            .map(|t| t.0.clone())
            .unwrap_or_else(|| "UTC".to_string());

        let parsed = parse_uri(&request.uri).map_err(|_| {
            ErrorData::invalid_params(
                self.i18n
                    .translate_args(&locale, "mcp-err-bad-uri", &[("uri", &request.uri)]),
                None,
            )
        })?;

        let to_internal = |e: serde_json::Error| ErrorData::internal_error(e.to_string(), None);

        let json = match parsed {
            ResourceUri::Lists => {
                let data = domain::lists::list_all(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::ListDetail(id) => {
                let data = domain::lists::get_one(&self.pool, &id, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?
                    .ok_or_else(|| {
                        self.map_err(
                            McpError::Domain(domain::DomainError::NotFound("list")),
                            &locale,
                        )
                    })?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::ListItems { list_id, .. } => {
                let data = domain::items::list_for_list(&self.pool, &list_id, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::Containers => {
                let data = domain::containers::list_all(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::ContainerDetail(id) => {
                let data = domain::containers::get_one(&self.pool, &id, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::Tags { .. } => {
                let data = domain::tags::list_all(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::Today => {
                let today = today_in_tz(&timezone);
                let data = domain::items::by_date(&self.pool, &user_id, &today)
                    .await
                    .map_err(self.domain_err(&locale))?;
                let data: Vec<_> = data
                    .into_iter()
                    .map(|i| localize_item(i, &timezone))
                    .collect();
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::TimeSummary => {
                let data = domain::time_entries::list_all_for_user(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
        };

        Ok(ReadResourceResponse::Complete(ReadResourceResult::new(
            vec![ResourceContents::text(
                serde_json::to_string_pretty(&json).unwrap_or_default(),
                request.uri,
            )],
        )))
    }
}

pub(crate) fn today_in_tz(tz_str: &str) -> String {
    let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    chrono::Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

/// Encode a pagination offset as an opaque cursor.
///
/// The cursor is `URL_SAFE_NO_PAD` base64 of `o:<offset>`, so clients must not
/// interpret or construct it themselves; they only echo it back.
pub(crate) fn encode_cursor(offset: usize) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("o:{offset}"))
}

/// Decode a cursor produced by [`encode_cursor`], falling back to offset 0.
///
/// Anything that is not one of our own cursors (absent, invalid base64, invalid
/// UTF-8, missing `o:` prefix, non-numeric payload) is treated as "start over".
pub(crate) fn decode_cursor(cursor: Option<&str>) -> usize {
    use base64::Engine as _;
    let Some(cursor) = cursor else {
        return 0;
    };
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(cursor) else {
        return 0;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return 0;
    };
    let Some(rest) = text.strip_prefix("o:") else {
        return 0;
    };
    rest.parse().unwrap_or(0)
}

pub(crate) fn localize_item(mut item: domain::items::Item, tz: &str) -> domain::items::Item {
    item.created_at = kartoteka_shared::date_utils::format_datetime_in_tz(&item.created_at, tz);
    item.updated_at = kartoteka_shared::date_utils::format_datetime_in_tz(&item.updated_at, tz);
    item
}
