# MCP Tools + Resources — Design Spec

Parent: `00-overview.md`
Crate: `crates/mcp/` (depends: shared, domain, i18n, rmcp, schemars)

## Architecture

Reads as **resources** (data discovery), mutations as **tools**. Claude Code browses via resources, acts via tools. All call `domain::` (never db::).

User ID extracted per-request from `http::request::Parts` extensions (injected by bearer auth middleware in oauth/ crate). Not stored in struct.

```rust
fn extract_user_id(parts: &http::request::Parts) -> Result<String, McpError> {
    parts.extensions.get::<UserId>()
        .map(|u| u.0.clone())
        .ok_or_else(|| McpError::new("unauthorized"))
}
```

## Resources (8)

| URI | Description |
|-----|-------------|
| `kartoteka://lists` | All lists for user |
| `kartoteka://lists/{list_id}` | List detail with features |
| `kartoteka://lists/{list_id}/items` | Items in a list |
| `kartoteka://containers` | All containers |
| `kartoteka://containers/{container_id}` | Container detail with children + progress |
| `kartoteka://tags` | All tags (all types: tag, location, priority) |
| `kartoteka://today` | Items due today (user timezone) |
| `kartoteka://time/summary` | Time tracking summary (today/week) |

Resource templates for dynamic URIs (`list_resource_templates`). URI parsing routes to domain:: functions.

## Tools (11)

| Tool | Description |
|------|-------------|
| `search_items` | FTS5 full-text search (parametric) |
| `create_item` | Create item in a list |
| `update_item` | Update item fields |
| `add_comment` | Add comment (author_type: assistant, optional persona) |
| `add_relation` | Create blocks/relates_to relation |
| `remove_relation` | Remove relation |
| `start_timer` | Start time tracking (auto-stops previous) |
| `stop_timer` | Stop running timer |
| `log_time` | Log retrospective time entry |
| `create_list_from_template` | Create list from template(s) |
| `save_as_template` | Snapshot list as template |

```rust
pub struct KartotekaServer {
    pool: SqlitePool,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl KartotekaServer {
    #[tool(name = "create_item", description = "Create a new item in a list")]
    async fn create_item(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(p): Parameters<CreateItemParams>,
    ) -> Result<CallToolResult, McpError> {
        let user_id = extract_user_id(&parts)?;
        let item = domain::items::create(&self.pool, &user_id, &p.list_id, &p.into()).await?;
        Ok(CallToolResult::success(serde_json::to_value(item)?))
    }
    // other tools — same pattern
}
```

## i18n

Tool descriptions and error messages in user's locale. Priority: user preferences → Accept-Language header → "en".

## Timezone

Date-related responses in user's timezone via domain:: (chrono-tz).

## Crate structure

```
crates/mcp/src/
  lib.rs          — re-exports
  server.rs       — KartotekaServer struct, tool_router, resource handlers
  tools/          — per-tool params (schemars derives)
  resources.rs    — URI parsing, resource template registration
```

## Mounting

```rust
let mcp_service = StreamableHttpService::new(
    move || Ok(KartotekaServer::new(pool.clone())),
    Arc::new(LocalSessionManager::default()),
    mcp_config,
);

// In server/main.rs, protected by bearer middleware from oauth/ crate
Router::new().nest_service("/mcp", mcp_service)
```

## Testing

Integration tests: create user + data → call tool → verify response. In-memory SQLite.
