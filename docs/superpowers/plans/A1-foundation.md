# A1: Foundation — Scaffold + Migration + Shared Types

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold all 10 crates with empty stubs, create the consolidated SQLite migration, implement `db::create_pool` with WAL/pragmas, shared `FlexDate` type, sqlx row types, and test helpers — so `cargo check --workspace` compiles and migration+pool work.

**Architecture:** Monorepo with 10 crates under `crates/`. `shared` is a leaf (no sqlx). `db` depends on `shared` and provides pool, migration, sqlx types, FlexDate Encode/Decode. All other crates are empty stubs (just `lib.rs` with a comment). Existing `api` and `frontend` crates remain in the workspace but are NOT modified — they'll be removed in a later plan.

**Tech Stack:** Rust 2024 edition, sqlx 0.8 (sqlite, runtime-tokio, chrono), chrono, thiserror, uuid, serde, schemars, tokio (dev-dependency for tests)

**Deferred to D1:** `[[workspace.metadata.leptos]]` in root Cargo.toml — requires cargo-leptos + leptos deps, not needed until SSR shell.
**Deferred to B1-B5:** `From<RowType> for DomainType` conversions — domain types defined in Phase B.

---

## File Structure

```
Cargo.toml                          — MODIFY: add 6 new workspace members, workspace deps
crates/shared/Cargo.toml            — MODIFY: add chrono, schemars deps
crates/shared/src/lib.rs            — MODIFY: add FlexDate, Icon, new types module
crates/shared/src/types.rs          — CREATE: FlexDate enum, Icon, constants
crates/shared/src/tests/flex_date.rs — CREATE: FlexDate unit tests
crates/db/Cargo.toml                — CREATE: sqlx, shared, uuid, thiserror deps
crates/db/src/lib.rs                — CREATE: create_pool, run_migrations, DbError, re-exports
crates/db/src/types.rs              — CREATE: sqlx row structs, FlexDate Encode/Decode
crates/db/src/test_helpers.rs       — CREATE: test_pool(), create_test_user()
crates/db/migrations/               — CREATE: directory
crates/db/migrations/001_init.sql   — CREATE: consolidated ~20 tables + indexes + FTS5 + triggers
crates/domain/Cargo.toml            — CREATE: stub
crates/domain/src/lib.rs            — CREATE: DomainError, re-exports (empty modules)
crates/auth/Cargo.toml              — CREATE: stub
crates/auth/src/lib.rs              — CREATE: empty stub
crates/mcp/Cargo.toml               — CREATE: stub
crates/mcp/src/lib.rs               — CREATE: empty stub
crates/oauth/Cargo.toml             — CREATE: stub
crates/oauth/src/lib.rs             — CREATE: empty stub
crates/jobs/Cargo.toml              — CREATE: stub
crates/jobs/src/lib.rs              — CREATE: empty stub
crates/frontend-v2/Cargo.toml       — CREATE: stub (frontend-v2 to avoid conflict with existing frontend/)
crates/frontend-v2/src/lib.rs       — CREATE: empty stub
crates/server/Cargo.toml            — CREATE: stub
crates/server/src/lib.rs            — CREATE: empty stub
```

**Note:** The existing `crates/frontend/` and `crates/api/` stay as-is — they remain workspace members and keep compiling. The new SSR frontend crate is `crates/frontend-v2/` to avoid collision; it will be renamed to `frontend` when the old one is removed.

---

### Task 1: Scaffold workspace Cargo.toml + 6 empty crate stubs

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/db/Cargo.toml`, `crates/db/src/lib.rs`
- Create: `crates/domain/Cargo.toml`, `crates/domain/src/lib.rs`
- Create: `crates/auth/Cargo.toml`, `crates/auth/src/lib.rs`
- Create: `crates/mcp/Cargo.toml`, `crates/mcp/src/lib.rs`
- Create: `crates/oauth/Cargo.toml`, `crates/oauth/src/lib.rs`
- Create: `crates/jobs/Cargo.toml`, `crates/jobs/src/lib.rs`
- Create: `crates/frontend-v2/Cargo.toml`, `crates/frontend-v2/src/lib.rs`
- Create: `crates/server/Cargo.toml`, `crates/server/src/lib.rs`

- [ ] **Step 1: Add workspace members and workspace dependencies to root Cargo.toml**

Add the 6 new members to the `[workspace]` section and add shared workspace dependencies:

```toml
[workspace]
resolver = "2"
members = [
    "crates/shared",
    "crates/api",
    "crates/frontend",
    "crates/i18n",
    "crates/db",
    "crates/domain",
    "crates/auth",
    "crates/mcp",
    "crates/oauth",
    "crates/jobs",
    "crates/frontend-v2",
    "crates/server",
]

[workspace.package]
version = "0.1.1"
edition = "2024"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"

# (keep existing [workspace.lints.clippy] section unchanged)
```

- [ ] **Step 2: Create all 8 new crate stubs**

Each stub crate gets a `Cargo.toml` and `src/lib.rs`. Here are the Cargo.tomls (lib.rs files are all initially empty `// Stub — implementation in later plans`):

**`crates/db/Cargo.toml`:**
```toml
[package]
name = "kartoteka-db"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
sqlx.workspace = true
chrono.workspace = true
thiserror.workspace = true
uuid.workspace = true
tracing.workspace = true

[dev-dependencies]
tokio.workspace = true
```

**`crates/domain/Cargo.toml`:**
```toml
[package]
name = "kartoteka-domain"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-db = { path = "../db" }
thiserror.workspace = true
```

**`crates/auth/Cargo.toml`:**
```toml
[package]
name = "kartoteka-auth"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-domain = { path = "../domain" }
```

**`crates/mcp/Cargo.toml`:**
```toml
[package]
name = "kartoteka-mcp"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-domain = { path = "../domain" }
```

**`crates/oauth/Cargo.toml`:**
```toml
[package]
name = "kartoteka-oauth"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-domain = { path = "../domain" }
kartoteka-auth = { path = "../auth" }
```

**`crates/jobs/Cargo.toml`:**
```toml
[package]
name = "kartoteka-jobs"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-domain = { path = "../domain" }
```

**`crates/frontend-v2/Cargo.toml`:**
```toml
[package]
name = "kartoteka-frontend-v2"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
```

**`crates/server/Cargo.toml`:**
```toml
[package]
name = "kartoteka-server"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-db = { path = "../db" }
kartoteka-domain = { path = "../domain" }
kartoteka-auth = { path = "../auth" }
kartoteka-mcp = { path = "../mcp" }
kartoteka-oauth = { path = "../oauth" }
kartoteka-jobs = { path = "../jobs" }
kartoteka-frontend-v2 = { path = "../frontend-v2" }
```

Each `src/lib.rs`:
```rust
// Stub — implementation in later plans
```

- [ ] **Step 3: Run `cargo check --workspace` to verify all stubs compile**

Run: `cargo check --workspace`
Expected: compiles with 0 errors. Warnings about unused deps are fine for now.

- [ ] **Step 4: Commit scaffold**

```bash
git add Cargo.toml crates/db/ crates/domain/ crates/auth/ crates/mcp/ crates/oauth/ crates/jobs/ crates/frontend-v2/ crates/server/
git commit -m "feat(a1): scaffold 8 new crates with workspace deps"
```

---

### Task 2: FlexDate in shared/ + unit tests

**Files:**
- Create: `crates/shared/src/types.rs`
- Create: `crates/shared/src/tests/flex_date.rs`
- Modify: `crates/shared/src/lib.rs`
- Modify: `crates/shared/Cargo.toml`

- [ ] **Step 1: Add chrono + schemars to shared Cargo.toml**

Add to `crates/shared/Cargo.toml` `[dependencies]`:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
schemars = "0.8"
```

- [ ] **Step 2: Write failing FlexDate tests**

Create `crates/shared/src/tests/flex_date.rs`:
```rust
use crate::types::FlexDate;
use chrono::NaiveDate;

#[test]
fn day_roundtrip_serde() {
    let d = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(json, "\"2026-05-15\"");
    let parsed: FlexDate = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, d);
}

#[test]
fn week_roundtrip_serde() {
    let w = FlexDate::Week(2026, 20);
    let json = serde_json::to_string(&w).unwrap();
    assert_eq!(json, "\"2026-W20\"");
    let parsed: FlexDate = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, w);
}

#[test]
fn month_roundtrip_serde() {
    let m = FlexDate::Month(2026, 5);
    let json = serde_json::to_string(&m).unwrap();
    assert_eq!(json, "\"2026-05\"");
    let parsed: FlexDate = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, m);
}

#[test]
fn day_start_end_equal() {
    let d = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    assert_eq!(d.start(), d.end());
}

#[test]
fn week_span_is_7_days() {
    let w = FlexDate::Week(2026, 20);
    let span = w.end().signed_duration_since(w.start()).num_days();
    assert_eq!(span, 6); // Mon-Sun inclusive = 6 day difference
}

#[test]
fn month_span() {
    let m = FlexDate::Month(2026, 2);
    assert_eq!(m.start(), NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
    assert_eq!(m.end(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
}

#[test]
fn is_fuzzy() {
    let day = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    let week = FlexDate::Week(2026, 1);
    let month = FlexDate::Month(2026, 1);
    assert!(!day.is_fuzzy());
    assert!(week.is_fuzzy());
    assert!(month.is_fuzzy());
}

#[test]
fn matches_day_exact() {
    let d = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    assert!(d.matches_day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap()));
    assert!(!d.matches_day(NaiveDate::from_ymd_opt(2026, 5, 16).unwrap()));
}

#[test]
fn matches_day_week_range() {
    let w = FlexDate::Week(2026, 20);
    let start = w.start();
    let end = w.end();
    assert!(w.matches_day(start));
    assert!(w.matches_day(end));
    assert!(!w.matches_day(start - chrono::Duration::days(1)));
}

#[test]
fn display_formats() {
    let d = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    assert_eq!(d.to_string(), "2026-05-15");
    let w = FlexDate::Week(2026, 5);
    assert_eq!(w.to_string(), "2026-W05");
    let m = FlexDate::Month(2026, 5);
    assert_eq!(m.to_string(), "2026-05");
}

#[test]
fn parse_from_str() {
    use std::str::FromStr;
    assert_eq!(
        FlexDate::from_str("2026-05-15").unwrap(),
        FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap())
    );
    assert_eq!(FlexDate::from_str("2026-W20").unwrap(), FlexDate::Week(2026, 20));
    assert_eq!(FlexDate::from_str("2026-05").unwrap(), FlexDate::Month(2026, 5));
    assert!(FlexDate::from_str("invalid").is_err());
}
```

Add to `crates/shared/src/tests/mod.rs` (the existing test module — add the new submodule):
```rust
mod flex_date;
```

- [ ] **Step 3: Run tests — verify they fail**

Run: `cargo test -p kartoteka-shared flex_date`
Expected: compilation errors — `types` module doesn't exist yet.

- [ ] **Step 4: Implement FlexDate in `crates/shared/src/types.rs`**

Create `crates/shared/src/types.rs`:
```rust
use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Flexible date with day, week, or month precision.
///
/// SQLite storage: TEXT column.
/// - 10 chars "YYYY-MM-DD" → Day
/// - 8 chars "YYYY-Wnn" → Week (ISO week)
/// - 7 chars "YYYY-MM" → Month
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlexDate {
    Day(NaiveDate),
    Week(u16, u8),
    Month(u16, u8),
}

impl FlexDate {
    /// First day of the period.
    pub fn start(&self) -> NaiveDate {
        match self {
            FlexDate::Day(d) => *d,
            FlexDate::Week(year, week) => {
                NaiveDate::from_isoywd_opt(i32::from(*year), u32::from(*week), chrono::Weekday::Mon)
                    .expect("valid ISO week")
            }
            FlexDate::Month(year, month) => {
                NaiveDate::from_ymd_opt(i32::from(*year), u32::from(*month), 1)
                    .expect("valid month")
            }
        }
    }

    /// Last day of the period.
    pub fn end(&self) -> NaiveDate {
        match self {
            FlexDate::Day(d) => *d,
            FlexDate::Week(year, week) => {
                NaiveDate::from_isoywd_opt(i32::from(*year), u32::from(*week), chrono::Weekday::Sun)
                    .expect("valid ISO week")
            }
            FlexDate::Month(year, month) => {
                // Last day: first day of next month - 1
                let (y, m) = if *month == 12 {
                    (i32::from(*year) + 1, 1)
                } else {
                    (i32::from(*year), u32::from(*month) + 1)
                };
                NaiveDate::from_ymd_opt(y, m, 1)
                    .expect("valid date")
                    .pred_opt()
                    .expect("valid pred")
            }
        }
    }

    /// True if not day-level precision.
    pub fn is_fuzzy(&self) -> bool {
        !matches!(self, FlexDate::Day(_))
    }

    /// Check if a specific day falls within this date's range.
    pub fn matches_day(&self, day: NaiveDate) -> bool {
        day >= self.start() && day <= self.end()
    }
}

impl fmt::Display for FlexDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlexDate::Day(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            FlexDate::Week(year, week) => write!(f, "{year}-W{week:02}"),
            FlexDate::Month(year, month) => write!(f, "{year}-{month:02}"),
        }
    }
}

/// Parse from string: "2026-05-15" | "2026-W20" | "2026-05"
impl FromStr for FlexDate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
            let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|e| format!("invalid day date: {e}"))?;
            Ok(FlexDate::Day(d))
        } else if s.contains("-W") {
            let parts: Vec<&str> = s.split("-W").collect();
            if parts.len() != 2 {
                return Err(format!("invalid week date: {s}"));
            }
            let year: u16 = parts[0].parse().map_err(|e| format!("invalid year: {e}"))?;
            let week: u8 = parts[1].parse().map_err(|e| format!("invalid week: {e}"))?;
            Ok(FlexDate::Week(year, week))
        } else if s.len() == 7 && s.as_bytes()[4] == b'-' {
            let parts: Vec<&str> = s.split('-').collect();
            let year: u16 = parts[0].parse().map_err(|e| format!("invalid year: {e}"))?;
            let month: u8 = parts[1].parse().map_err(|e| format!("invalid month: {e}"))?;
            Ok(FlexDate::Month(year, month))
        } else {
            Err(format!("unrecognized date format: {s}"))
        }
    }
}

impl Serialize for FlexDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for FlexDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        FlexDate::from_str(&s).map_err(serde::de::Error::custom)
    }
}
```

- [ ] **Step 5: Wire types module into shared lib.rs**

Add to the top of `crates/shared/src/lib.rs` (before existing code):
```rust
pub mod types;
```

- [ ] **Step 6: Run tests — verify they pass**

Run: `cargo test -p kartoteka-shared flex_date -- --nocapture`
Expected: all 12 FlexDate tests pass.

- [ ] **Step 7: Commit FlexDate**

```bash
git add crates/shared/
git commit -m "feat(a1): add FlexDate enum with serde, chrono, parse/display"
```

---

### Task 3: Consolidated SQLite migration

**Files:**
- Create: `crates/db/migrations/001_init.sql`

- [ ] **Step 1: Create migration directory**

```bash
mkdir -p crates/db/migrations
```

- [ ] **Step 2: Write the consolidated migration**

Create `crates/db/migrations/001_init.sql`:
```sql
-- Kartoteka consolidated schema
-- All tables, indexes, FTS5, triggers in one migration.

-- Auth
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    name TEXT,
    avatar_url TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS auth_methods (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    provider TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    credential TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_id)
);

CREATE TABLE IF NOT EXISTS totp_secrets (
    user_id TEXT PRIMARY KEY REFERENCES users(id),
    secret TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS personal_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'full',
    last_used_at TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Server config
CREATE TABLE IF NOT EXISTS server_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

INSERT OR IGNORE INTO server_config (key, value) VALUES ('registration_enabled', 'true');

-- OAuth
CREATE TABLE IF NOT EXISTS oauth_clients (
    client_id TEXT PRIMARY KEY,
    client_name TEXT,
    redirect_uris TEXT NOT NULL,
    grant_types TEXT NOT NULL,
    token_endpoint_auth_method TEXT DEFAULT 'none',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE IF NOT EXISTS oauth_authorization_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    code_challenge_method TEXT NOT NULL DEFAULT 'S256',
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE IF NOT EXISTS oauth_refresh_tokens (
    token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- App data
CREATE TABLE IF NOT EXISTS containers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    status TEXT,
    parent_container_id TEXT REFERENCES containers(id),
    position INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS lists (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    list_type TEXT NOT NULL DEFAULT 'checklist',
    parent_list_id TEXT REFERENCES lists(id),
    position INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    container_id TEXT REFERENCES containers(id),
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS items (
    id TEXT PRIMARY KEY,
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    completed INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    quantity INTEGER,
    actual_quantity INTEGER,
    unit TEXT,
    start_date TEXT,
    start_time TEXT,
    deadline TEXT,
    deadline_time TEXT,
    hard_deadline TEXT,
    estimated_duration INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    color TEXT,
    parent_tag_id TEXT REFERENCES tags(id),
    tag_type TEXT NOT NULL DEFAULT 'tag',
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
);

CREATE TABLE IF NOT EXISTS list_tags (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, tag_id)
);

CREATE TABLE IF NOT EXISTS container_tags (
    container_id TEXT NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (container_id, tag_id)
);

CREATE TABLE IF NOT EXISTS list_features (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL,
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);

CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

-- Comments (polymorphic)
CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    content TEXT NOT NULL,
    author_type TEXT NOT NULL DEFAULT 'user',
    author_name TEXT,
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Relations (polymorphic)
CREATE TABLE IF NOT EXISTS entity_relations (
    id TEXT PRIMARY KEY,
    from_type TEXT NOT NULL,
    from_id TEXT NOT NULL,
    to_type TEXT NOT NULL,
    to_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(from_type, from_id, to_type, to_id, relation_type)
) STRICT;

-- Time tracking
CREATE TABLE IF NOT EXISTS time_entries (
    id TEXT PRIMARY KEY,
    item_id TEXT REFERENCES items(id) ON DELETE SET NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    description TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration INTEGER,
    source TEXT NOT NULL,
    mode TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Templates
CREATE TABLE IF NOT EXISTS templates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE IF NOT EXISTS template_items (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    quantity INTEGER,
    unit TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE IF NOT EXISTS template_tags (
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (template_id, tag_id)
);

-- FTS5
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(title, description, content=items, content_rowid=rowid);
CREATE VIRTUAL TABLE IF NOT EXISTS comments_fts USING fts5(content, content=comments, content_rowid=rowid);

-- FTS5 sync triggers
CREATE TRIGGER IF NOT EXISTS items_fts_insert AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
END;
CREATE TRIGGER IF NOT EXISTS items_fts_update AFTER UPDATE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, title, description) VALUES ('delete', old.rowid, old.title, old.description);
    INSERT INTO items_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
END;
CREATE TRIGGER IF NOT EXISTS items_fts_delete AFTER DELETE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, title, description) VALUES ('delete', old.rowid, old.title, old.description);
END;
CREATE TRIGGER IF NOT EXISTS comments_fts_insert AFTER INSERT ON comments BEGIN
    INSERT INTO comments_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER IF NOT EXISTS comments_fts_update AFTER UPDATE ON comments BEGIN
    INSERT INTO comments_fts(comments_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO comments_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER IF NOT EXISTS comments_fts_delete AFTER DELETE ON comments BEGIN
    INSERT INTO comments_fts(comments_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

-- Indexes
CREATE INDEX IF NOT EXISTS idx_items_list_id ON items(list_id);
CREATE INDEX IF NOT EXISTS idx_items_deadline ON items(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_items_start_date ON items(start_date) WHERE start_date IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_lists_user_id ON lists(user_id);
CREATE INDEX IF NOT EXISTS idx_lists_pinned ON lists(user_id, pinned) WHERE pinned = 1;
CREATE INDEX IF NOT EXISTS idx_lists_container ON lists(container_id) WHERE container_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_containers_user_id ON containers(user_id);
CREATE INDEX IF NOT EXISTS idx_containers_pinned ON containers(user_id, pinned) WHERE pinned = 1;
CREATE INDEX IF NOT EXISTS idx_auth_methods_user_provider ON auth_methods(user_id, provider);
CREATE INDEX IF NOT EXISTS idx_tags_user_id ON tags(user_id);
CREATE INDEX IF NOT EXISTS idx_tags_user_type ON tags(user_id, tag_type);
CREATE INDEX IF NOT EXISTS idx_comments_entity ON comments(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_relations_from ON entity_relations(from_type, from_id);
CREATE INDEX IF NOT EXISTS idx_relations_to ON entity_relations(to_type, to_id);
CREATE INDEX IF NOT EXISTS idx_time_entries_item ON time_entries(item_id) WHERE item_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_time_entries_user_unassigned ON time_entries(user_id) WHERE item_id IS NULL;
CREATE INDEX IF NOT EXISTS idx_template_items_template ON template_items(template_id);
CREATE INDEX IF NOT EXISTS idx_personal_tokens_user ON personal_tokens(user_id);
```

- [ ] **Step 3: Commit migration**

```bash
git add crates/db/migrations/
git commit -m "feat(a1): consolidated SQLite migration — 20 tables, FTS5, indexes"
```

---

### Task 4: `db::create_pool` + `run_migrations` + `DbError`

**Files:**
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: Write failing test for pool creation + migration**

Add to `crates/db/src/lib.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_connects_and_migrates() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify a table exists by querying it
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn migration_creates_server_config_default() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let row: (String,) =
            sqlx::query_as("SELECT value FROM server_config WHERE key = 'registration_enabled'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "true");
    }

    #[tokio::test]
    async fn migration_creates_fts5_tables() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        // FTS5 tables should exist — inserting into items should trigger FTS sync
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'test@test.com')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'Test List')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO items (id, list_id, title, description) VALUES ('i1', 'l1', 'Buy milk', 'whole milk')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // FTS search should find the item
        let results: Vec<(String,)> =
            sqlx::query_as("SELECT title FROM items_fts WHERE items_fts MATCH 'milk'")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "Buy milk");
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test -p kartoteka-db`
Expected: compilation error — `create_pool` and `run_migrations` don't exist yet.

- [ ] **Step 3: Implement `create_pool`, `run_migrations`, `DbError`**

Replace `crates/db/src/lib.rs` with:
```rust
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::str::FromStr;

pub mod test_helpers;
pub mod types;

/// Database error type.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("not found: {0}")]
    NotFound(&'static str),
}

/// Create a SQLite connection pool with WAL mode, pragmas, and tuning.
///
/// Pass `":memory:"` for in-memory (tests), or a file path for persistent storage.
pub async fn create_pool(url: &str) -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .min_connections(2)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 5000")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA mmap_size = 268435456")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA optimize = 0x10002")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await?;

    Ok(pool)
}

/// Run all embedded migrations against the pool.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

// Re-export pool type for consumers
pub use sqlx::sqlite::SqlitePool;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_connects_and_migrates() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn migration_creates_server_config_default() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let row: (String,) =
            sqlx::query_as("SELECT value FROM server_config WHERE key = 'registration_enabled'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "true");
    }

    #[tokio::test]
    async fn migration_creates_fts5_tables() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'test@test.com')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'Test List')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO items (id, list_id, title, description) VALUES ('i1', 'l1', 'Buy milk', 'whole milk')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let results: Vec<(String,)> =
            sqlx::query_as("SELECT title FROM items_fts WHERE items_fts MATCH 'milk'")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "Buy milk");
    }
}
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `cargo test -p kartoteka-db`
Expected: all 3 tests pass.

- [ ] **Step 5: Commit pool + migration runner**

```bash
git add crates/db/
git commit -m "feat(a1): db::create_pool with WAL/pragmas + run_migrations"
```

---

### Task 5: sqlx row types + FlexDate Encode/Decode in db/

**Files:**
- Create: `crates/db/src/types.rs`

- [ ] **Step 1: Write failing test for FlexDate sqlx roundtrip**

Add to `crates/db/src/types.rs` (at the bottom, inside `#[cfg(test)]`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;
    use kartoteka_shared::types::FlexDate;

    #[tokio::test]
    async fn flex_date_roundtrip_day() {
        let pool = test_pool().await;
        let date = FlexDate::Day(chrono::NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, start_date) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT start_date FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-05-15");

        let decoded: (FlexDate,) = sqlx::query_as("SELECT start_date FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(decoded.0, date);
    }

    #[tokio::test]
    async fn flex_date_roundtrip_week() {
        let pool = test_pool().await;
        let date = FlexDate::Week(2026, 20);

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, deadline) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT deadline FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-W20");
    }

    #[tokio::test]
    async fn flex_date_roundtrip_month() {
        let pool = test_pool().await;
        let date = FlexDate::Month(2026, 5);

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, hard_deadline) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT hard_deadline FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-05");
    }

    #[tokio::test]
    async fn user_row_from_db() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email, name, role) VALUES ('u1', 'a@b.com', 'Alice', 'admin')")
            .execute(&pool).await.unwrap();

        let user: UserRow = sqlx::query_as("SELECT id, email, name, avatar_url, role, created_at, updated_at FROM users WHERE id = 'u1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(user.id, "u1");
        assert_eq!(user.email, "a@b.com");
        assert_eq!(user.name.as_deref(), Some("Alice"));
        assert_eq!(user.role, "admin");
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test -p kartoteka-db types`
Expected: compilation error — types module empty, `test_pool` doesn't exist.

- [ ] **Step 3: Implement test_helpers first (needed by type tests)**

Create `crates/db/src/test_helpers.rs`:
```rust
use crate::{create_pool, run_migrations};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

/// Create an in-memory SQLite pool with all migrations applied. For tests only.
pub async fn test_pool() -> SqlitePool {
    let pool = create_pool(":memory:").await.expect("test pool creation");
    run_migrations(&pool).await.expect("test migrations");
    pool
}

/// Insert a minimal test user and return their ID.
pub async fn create_test_user(pool: &SqlitePool) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (id, email, name, role) VALUES (?, ?, 'Test User', 'user')")
        .bind(&id)
        .bind(format!("{}@test.local", &id[..8]))
        .execute(pool)
        .await
        .expect("create_test_user");
    id
}
```

- [ ] **Step 4: Implement FlexDate Encode/Decode + row types**

Write `crates/db/src/types.rs`:
```rust
use kartoteka_shared::types::FlexDate;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Sqlite, Type};
use std::str::FromStr;

// --- FlexDate sqlx integration ---

impl Type<Sqlite> for FlexDate {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <String as Type<Sqlite>>::compatible(ty)
    }
}

impl Encode<'_, Sqlite> for FlexDate {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'_>>) -> Result<IsNull, BoxDynError> {
        let s = self.to_string();
        args.push(SqliteArgumentValue::Text(s.into()));
        Ok(IsNull::No)
    }
}

impl Decode<'_, Sqlite> for FlexDate {
    fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        FlexDate::from_str(&s).map_err(|e| e.into())
    }
}

// --- sqlx row types ---
// These are the DB-level row structs used by sqlx::query_as.
// Domain types in shared/ are separate — From conversions bridge them.

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ContainerRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ListRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i32,
    pub archived: bool,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ItemRow {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<FlexDate>,
    pub start_time: Option<String>,
    pub deadline: Option<FlexDate>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<FlexDate>,
    pub estimated_duration: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TagRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    pub tag_type: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ListFeatureRow {
    pub list_id: String,
    pub feature_name: String,
    pub config: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserSettingRow {
    pub user_id: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CommentRow {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    pub author_type: String,
    pub author_name: Option<String>,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EntityRelationRow {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub relation_type: String,
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TimeEntryRow {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i32>,
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateItemRow {
    pub id: String,
    pub template_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PersonalTokenRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub scope: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;
    use kartoteka_shared::types::FlexDate;

    #[tokio::test]
    async fn flex_date_roundtrip_day() {
        let pool = test_pool().await;
        let date = FlexDate::Day(chrono::NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, start_date) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT start_date FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-05-15");

        let decoded: (FlexDate,) = sqlx::query_as("SELECT start_date FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(decoded.0, date);
    }

    #[tokio::test]
    async fn flex_date_roundtrip_week() {
        let pool = test_pool().await;
        let date = FlexDate::Week(2026, 20);

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, deadline) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT deadline FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-W20");
    }

    #[tokio::test]
    async fn flex_date_roundtrip_month() {
        let pool = test_pool().await;
        let date = FlexDate::Month(2026, 5);

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, hard_deadline) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT hard_deadline FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-05");
    }

    #[tokio::test]
    async fn user_row_from_db() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email, name, role) VALUES ('u1', 'a@b.com', 'Alice', 'admin')")
            .execute(&pool).await.unwrap();

        let user: UserRow = sqlx::query_as("SELECT id, email, name, avatar_url, role, created_at, updated_at FROM users WHERE id = 'u1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(user.id, "u1");
        assert_eq!(user.email, "a@b.com");
        assert_eq!(user.name.as_deref(), Some("Alice"));
        assert_eq!(user.role, "admin");
    }
}
```

- [ ] **Step 5: Run tests — verify they pass**

Run: `cargo test -p kartoteka-db types`
Expected: all 4 type tests pass.

- [ ] **Step 6: Commit types**

```bash
git add crates/db/src/types.rs crates/db/src/test_helpers.rs
git commit -m "feat(a1): sqlx row types, FlexDate Encode/Decode, test helpers"
```

---

### Task 6: DomainError in domain/ + test_helpers integration test

**Files:**
- Modify: `crates/domain/src/lib.rs`
- Modify: `crates/domain/Cargo.toml`

- [ ] **Step 1: Write failing test for DomainError + test_pool integration**

```rust
// In crates/domain/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn test_pool_and_create_user_work() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        assert!(!user_id.is_empty());
        assert_eq!(user_id.len(), 36); // UUID v4 length

        // Verify user exists
        let row: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "user");
    }

    #[test]
    fn domain_error_from_db_error() {
        let db_err = kartoteka_db::DbError::NotFound("item");
        let domain_err: DomainError = db_err.into();
        assert!(matches!(domain_err, DomainError::Db(_)));
    }
}
```

- [ ] **Step 2: Run test — verify it fails**

Run: `cargo test -p kartoteka-domain`
Expected: compilation error — `DomainError` doesn't exist.

- [ ] **Step 3: Implement DomainError**

Update `crates/domain/Cargo.toml` to add missing deps:
```toml
[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-db = { path = "../db" }
thiserror.workspace = true
sqlx.workspace = true

[dev-dependencies]
tokio.workspace = true
```

Write `crates/domain/src/lib.rs`:
```rust
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("validation: {0}")]
    Validation(&'static str),
    #[error("feature required: {0}")]
    FeatureRequired(&'static str),
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Db(#[from] kartoteka_db::DbError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn test_pool_and_create_user_work() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        assert!(!user_id.is_empty());
        assert_eq!(user_id.len(), 36);

        let row: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "user");
    }

    #[test]
    fn domain_error_from_db_error() {
        let db_err = kartoteka_db::DbError::NotFound("item");
        let domain_err: DomainError = db_err.into();
        assert!(matches!(domain_err, DomainError::Db(_)));
    }
}
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `cargo test -p kartoteka-domain`
Expected: both tests pass.

- [ ] **Step 5: Commit DomainError**

```bash
git add crates/domain/
git commit -m "feat(a1): DomainError with Db conversion + test helpers integration test"
```

---

### Task 7: Final verification — full workspace check + all tests

**Files:** None (verification only)

- [ ] **Step 1: Run `cargo check --workspace`**

Run: `cargo check --workspace`
Expected: 0 errors. Some warnings about unused deps in stub crates are acceptable.

- [ ] **Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: All existing tests (shared, i18n) still pass. New tests (shared/flex_date, db, domain) all pass. Total new tests: ~18.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 errors. Fix any clippy warnings before committing.

- [ ] **Step 4: Fix any clippy issues and commit if needed**

If clippy found issues, fix them and commit:
```bash
git add -A
git commit -m "fix(a1): clippy fixes"
```

---

## Summary

| Task | What | Tests | LOC (approx) |
|------|------|-------|---------------|
| 1 | Workspace + 8 crate stubs | cargo check | ~120 |
| 2 | FlexDate in shared/ | 12 unit tests | ~130 |
| 3 | Consolidated migration SQL | (tested via db) | ~180 |
| 4 | create_pool + run_migrations + DbError | 3 integration tests | ~80 |
| 5 | sqlx row types + FlexDate Encode/Decode | 4 integration tests | ~200 |
| 6 | DomainError + test helpers | 2 tests | ~40 |
| 7 | Final verification | full workspace check | 0 |
| **Total** | | **~21 tests** | **~750** |

**Deliverable:** `cargo check --workspace` compiles, migration runs, pool connects, FlexDate round-trips through SQLite, all tests green.
