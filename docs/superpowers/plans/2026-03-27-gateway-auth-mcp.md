# Gateway Worker (Auth + MCP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Hanko auth with Better Auth and add MCP server, unified behind a single TypeScript Gateway Worker that proxies to the existing Rust API Worker via CF service binding.

**Architecture:** Single Hono-based TypeScript Worker handles `/auth/*` (Better Auth), `/mcp/*` (MCP SDK + OAuth provider), and `/api/*` (proxy to Rust API Worker via service binding). The Rust API Worker drops its own auth and trusts `X-User-Id` from Gateway.

**Tech Stack:** TypeScript, Hono, Better Auth (D1 adapter), @modelcontextprotocol/sdk, @cloudflare/workers-oauth-provider, Cloudflare Workers, D1, KV

**Spec:** `docs/superpowers/specs/2026-03-27-gateway-auth-mcp-design.md`

---

## File Structure

### New files (gateway/)

```
gateway/
├── src/
│   ├── index.ts              — Hono app entry, route mounting, CORS
│   ├── auth.ts               — Better Auth instance config (D1, email+password, GitHub)
│   ├── middleware.ts          — session validation middleware, X-User-Id injection
│   ├── proxy.ts              — service binding proxy: forward /api/* to API Worker
│   ├── mcp/
│   │   ├── server.ts         — McpServer + OAuthProvider setup
│   │   └── tools.ts          — 5 MCP tool definitions (list_lists, get_list_items, create_list, add_item, toggle_item)
│   └── types.ts              — Env + Variables interfaces (D1, KV, API_WORKER bindings, userId context)
├── package.json              — dependencies: hono, better-auth, @modelcontextprotocol/sdk, @cloudflare/workers-oauth-provider
├── tsconfig.json
├── wrangler.toml             — D1 auth binding, KV binding, service binding to API Worker
└── drizzle.config.ts         — Drizzle config for Better Auth schema generation
```

### Modified files

```
crates/api/src/auth.rs        — rewrite: drop Hanko, add X-User-Id header check
crates/api/src/router.rs      — remove CORS, use new auth
crates/api/wrangler.toml      — add workers_dev = false, remove HANKO_API_URL dependency
crates/frontend/src/api/mod.rs — drop Hanko token, switch to cookie-based auth (credentials: "include")
crates/frontend/src/pages/login.rs — replace <hanko-auth> with custom login form
crates/frontend/src/components/nav.rs — update auth checks for cookie-based auth
crates/frontend/Trunk.toml    — add proxy config for /auth routes
justfile                       — update dev/build/deploy commands for gateway
.env                           — new vars: BETTER_AUTH_SECRET, BETTER_AUTH_URL, GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET
.github/workflows/ci.yml      — remove HANKO_API_URL env var
```

### Deleted files

```
crates/frontend/hanko-init.js.template — no longer needed
mcp/                                    — replaced by gateway/
```

---

## Task 1: Scaffold Gateway Worker

**Files:**
- Create: `gateway/package.json`
- Create: `gateway/tsconfig.json`
- Create: `gateway/wrangler.toml`
- Create: `gateway/src/types.ts`
- Create: `gateway/src/index.ts`

- [ ] **Step 1: Create `gateway/package.json`**

```json
{
  "name": "kartoteka-gateway",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "wrangler dev",
    "deploy": "wrangler deploy",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "hono": "^4.0.0",
    "better-auth": "^1.5.0",
    "@cloudflare/workers-oauth-provider": "^0.1.0",
    "@modelcontextprotocol/sdk": "^1.0.0"
  },
  "devDependencies": {
    "@cloudflare/workers-types": "^4.0.0",
    "typescript": "^5.5.0",
    "wrangler": "^4.76.0"
  }
}
```

- [ ] **Step 2: Create `gateway/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "types": ["@cloudflare/workers-types"]
  },
  "include": ["src"]
}
```

- [ ] **Step 3: Create `gateway/wrangler.toml`**

Use the same `account_id` as the API worker. D1 auth database and KV namespace IDs are placeholders — create them with `wrangler d1 create` and `wrangler kv namespace create` before first deploy.

```toml
name = "kartoteka-gateway"
main = "src/index.ts"
compatibility_date = "2025-01-01"
account_id = "6b703fac2c4f8b359e430bf11b8dde7f"

# Auth database (Better Auth tables: user, session, account, verification)
[[d1_databases]]
binding = "AUTH_DB"
database_name = "kartoteka-auth"
database_id = "TODO_CREATE_WITH_WRANGLER"
migrations_dir = "./migrations"

# OAuth token storage for MCP
[[kv_namespaces]]
binding = "OAUTH_KV"
id = "TODO_CREATE_WITH_WRANGLER"

# Service binding to Rust API Worker
[[services]]
binding = "API_WORKER"
service = "kartoteka-api"

# Local dev environment
[env.local]
name = "kartoteka-gateway-local"

[env.local.vars]
DEV_AUTH_USER_ID = "dev-user-00000000-0000-0000-0000-000000000001"
BETTER_AUTH_SECRET = "dev-secret-not-for-production"
BETTER_AUTH_URL = "http://localhost:8788"

[[env.local.d1_databases]]
binding = "AUTH_DB"
database_name = "kartoteka-gateway-local"
database_id = "00000000-0000-0000-0000-000000000000"
migrations_dir = "./migrations"

[[env.local.kv_namespaces]]
binding = "OAUTH_KV"
id = "00000000-0000-0000-0000-000000000000"

[[env.local.services]]
binding = "API_WORKER"
service = "kartoteka-api"
```

- [ ] **Step 4: Create `gateway/src/types.ts`**

```typescript
export interface Env {
  AUTH_DB: D1Database;
  OAUTH_KV: KVNamespace;
  API_WORKER: Fetcher;
  BETTER_AUTH_SECRET: string;
  BETTER_AUTH_URL: string;
  GITHUB_CLIENT_ID?: string;
  GITHUB_CLIENT_SECRET?: string;
  DEV_AUTH_USER_ID?: string;
  DEV_API_URL?: string;
}

export interface Variables {
  userId: string;
}
```

- [ ] **Step 5: Create minimal `gateway/src/index.ts`**

Start with a health check only — verify the worker deploys and runs.

```typescript
import { Hono } from "hono";
import { cors } from "hono/cors";
import type { Env, Variables } from "./types";

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

app.use("/api/*", cors());
app.use("/auth/*", cors());

app.get("/health", (c) => c.text("ok"));

export default app;
```

- [ ] **Step 6: Install dependencies and verify typecheck**

Run: `cd gateway && npm install && npm run typecheck`
Expected: clean install, no type errors

- [ ] **Step 7: Verify local dev**

Run: `cd gateway && npx wrangler dev --env local --local`
Then: `curl http://localhost:8788/health`
Expected: `ok`

- [ ] **Step 8: Commit**

```bash
git add gateway/
git commit -m "feat(gateway): scaffold TypeScript Gateway Worker with Hono"
```

---

## Task 2: Better Auth Setup

**Files:**
- Create: `gateway/src/auth.ts`
- Modify: `gateway/src/index.ts`

**Docs to check:** Better Auth Cloudflare D1 setup — use `@context7` for `better-auth` to get current D1 adapter API. Key: Better Auth auto-generates tables on first request or via CLI migration.

- [ ] **Step 1: Create `gateway/src/auth.ts`**

```typescript
import { betterAuth } from "better-auth";
import { d1Adapter } from "better-auth/adapters/d1";
import type { Env } from "./types";

export function createAuth(env: Env) {
  return betterAuth({
    database: d1Adapter(env.AUTH_DB),
    secret: env.BETTER_AUTH_SECRET,
    baseURL: env.BETTER_AUTH_URL,
    emailAndPassword: {
      enabled: true,
    },
    // GitHub social login — only enabled if credentials are set
    ...(env.GITHUB_CLIENT_ID && env.GITHUB_CLIENT_SECRET
      ? {
          socialProviders: {
            github: {
              clientId: env.GITHUB_CLIENT_ID,
              clientSecret: env.GITHUB_CLIENT_SECRET,
            },
          },
        }
      : {}),
  });
}
```

Note: Check `better-auth` docs via context7 for exact D1 adapter import path and config shape — API may have changed since last check.

- [ ] **Step 2: Mount Better Auth routes in `gateway/src/index.ts`**

Add the `/auth/*` route handler. Better Auth's `handler` takes a standard `Request` and returns a `Response`.

```typescript
import { createAuth } from "./auth";

// Add to app, after CORS middleware:
app.all("/auth/*", async (c) => {
  const auth = createAuth(c.env);
  return auth.handler(c.req.raw);
});
```

- [ ] **Step 3: Generate Better Auth D1 schema/migrations**

Better Auth can generate SQL migrations for D1. Check docs for exact CLI command — likely:

Run: `cd gateway && npx better-auth generate`

This creates the `user`, `session`, `account`, `verification` tables. Apply locally:

Run: `cd gateway && npx wrangler d1 migrations apply kartoteka-gateway-local --env local --local`

- [ ] **Step 4: Test signup flow locally**

Run: `cd gateway && npx wrangler dev --env local --local`

Test with curl:
```bash
curl -X POST http://localhost:8788/auth/api/sign-up/email \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"testpassword123","name":"Test User"}'
```

Expected: JSON response with user object and session token/cookie.

- [ ] **Step 5: Test signin flow locally**

```bash
curl -X POST http://localhost:8788/auth/api/sign-in/email \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"testpassword123"}' \
  -c cookies.txt
```

Expected: JSON response with session, `Set-Cookie` header.

- [ ] **Step 6: Commit**

```bash
git add gateway/src/auth.ts gateway/src/index.ts gateway/migrations/
git commit -m "feat(gateway): add Better Auth with email+password and D1"
```

---

## Task 3: Auth Middleware + API Proxy

**Files:**
- Create: `gateway/src/middleware.ts`
- Create: `gateway/src/proxy.ts`
- Modify: `gateway/src/index.ts`

- [ ] **Step 1: Create `gateway/src/middleware.ts`**

Middleware that validates session cookie and extracts user ID. For dev mode, bypasses auth if `DEV_AUTH_USER_ID` is set.

```typescript
import { createMiddleware } from "hono/factory";
import { createAuth } from "./auth";
import type { Env } from "./types";

export const authMiddleware = createMiddleware<{ Bindings: Env; Variables: Variables }>(
  async (c, next) => {
    // Dev bypass
    const devUserId = c.env.DEV_AUTH_USER_ID;
    if (devUserId) {
      c.set("userId", devUserId);
      return next();
    }

    const auth = createAuth(c.env);
    const session = await auth.api.getSession({
      headers: c.req.raw.headers,
    });

    if (!session?.user?.id) {
      return c.json({ error: "Unauthorized" }, 401);
    }

    c.set("userId", session.user.id);
    return next();
  }
);
```

Note: Check Better Auth docs for exact `getSession` API — it may be `auth.api.getSession({ headers })` or a different method. Verify via context7.

- [ ] **Step 2: Create `gateway/src/proxy.ts`**

Proxy `/api/*` requests to API Worker via service binding, injecting `X-User-Id`.

```typescript
import { Hono } from "hono";
import type { Env } from "./types";

const proxy = new Hono<{ Bindings: Env }>();

proxy.all("/*", async (c) => {
  const userId = c.get("userId") as string;

  // Build new headers, forwarding originals + adding X-User-Id
  const headers = new Headers(c.req.raw.headers);
  headers.set("X-User-Id", userId);

  // Forward to API Worker via service binding
  const url = new URL(c.req.url);
  const apiRequest = new Request(url.toString(), {
    method: c.req.method,
    headers,
    body: c.req.raw.body,
  });

  return c.env.API_WORKER.fetch(apiRequest);
});

export { proxy };
```

- [ ] **Step 3: Wire middleware and proxy into `gateway/src/index.ts`**

```typescript
import { Hono } from "hono";
import { cors } from "hono/cors";
import { createAuth } from "./auth";
import { authMiddleware } from "./middleware";
import { proxy } from "./proxy";
import type { Env, Variables } from "./types";

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

app.use("/api/*", cors());
app.use("/auth/*", cors());

app.get("/health", (c) => c.text("ok"));

// Auth routes (Better Auth handles signup/signin/signout etc.)
app.all("/auth/*", async (c) => {
  const auth = createAuth(c.env);
  return auth.handler(c.req.raw);
});

// API routes — require auth, then proxy to Rust API Worker
app.use("/api/*", authMiddleware);
app.route("/api", proxy);

export default app;
```

- [ ] **Step 4: Test auth + proxy locally**

Start both workers (Gateway on 8788, API on 8787):
```bash
cd crates/api && npx wrangler dev --env local --local --port 8787 &
cd gateway && npx wrangler dev --env local --local --port 8788
```

Test unauthenticated request:
```bash
curl http://localhost:8788/api/lists
```
Expected: 401 Unauthorized (or dev bypass response if DEV_AUTH_USER_ID is set)

Test with dev bypass (DEV_AUTH_USER_ID set in wrangler.toml local env):
```bash
curl http://localhost:8788/api/health
```
Expected: `ok`

Note: Service bindings don't work in local dev (`wrangler dev`). For local testing, the proxy will need to fall back to HTTP fetch to `localhost:8787`. Add a `DEV_API_URL` env var for local dev that the proxy uses instead of service binding when set.

- [ ] **Step 5: Add DEV_API_URL fallback to proxy.ts**

Update `gateway/src/types.ts` to add `DEV_API_URL?: string` and update `proxy.ts`:

```typescript
// In proxy.ts, replace the API_WORKER.fetch call:
if (c.env.DEV_API_URL) {
  // Local dev: use HTTP fetch instead of service binding
  const devUrl = new URL(c.req.url);
  devUrl.host = new URL(c.env.DEV_API_URL).host;
  devUrl.port = new URL(c.env.DEV_API_URL).port;
  const apiRequest = new Request(devUrl.toString(), {
    method: c.req.method,
    headers,
    body: c.req.raw.body,
  });
  return fetch(apiRequest);
}
return c.env.API_WORKER.fetch(apiRequest);
```

Add to `wrangler.toml` under `[env.local.vars]`:
```toml
DEV_API_URL = "http://localhost:8787"
```

- [ ] **Step 6: Commit**

```bash
git add gateway/src/middleware.ts gateway/src/proxy.ts gateway/src/index.ts gateway/src/types.ts gateway/wrangler.toml
git commit -m "feat(gateway): add auth middleware and API proxy with service binding"
```

---

## Task 4: Modify API Worker — Drop Hanko Auth

**Prerequisite:** Must be done before Task 5 (MCP tools test against API Worker via service binding).

**Files:**
- Modify: `crates/api/src/auth.rs`
- Modify: `crates/api/src/router.rs`
- Modify: `crates/api/wrangler.toml`

- [ ] **Step 1: Rewrite `crates/api/src/auth.rs`**

Replace Hanko JWT validation with `X-User-Id` header check.

```rust
use worker::*;

/// Returns the dev bypass user_id if `DEV_AUTH_USER_ID` Worker var is set.
/// Used only in local dev — never set in prod/dev deployments.
pub fn dev_bypass_user_id(env: &Env) -> Option<String> {
    env.var("DEV_AUTH_USER_ID")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
}

/// Extracts user_id from X-User-Id header set by Gateway Worker.
/// In production, the API Worker is only reachable via service binding
/// from the Gateway, which validates auth and injects this header.
pub fn user_id_from_gateway(req: &Request) -> Result<String> {
    req.headers()
        .get("X-User-Id")?
        .ok_or_else(|| Error::from("Missing X-User-Id header"))
}
```

- [ ] **Step 2: Update `crates/api/src/router.rs` — remove CORS, use new auth**

Replace the auth block and remove CORS:

```rust
use worker::*;

use crate::auth;
use crate::handlers::{containers, items, lists, tags};

pub async fn handle(req: Request, env: Env) -> Result<Response> {
    let path = req.path();
    if path == "/api/health" {
        return Response::ok("ok");
    }

    let user_id = if let Some(uid) = auth::dev_bypass_user_id(&env) {
        uid
    } else {
        auth::user_id_from_gateway(&req)?
    };

    let router = Router::with_data(user_id);
    router
        // ... all existing routes unchanged ...
        .run(req, env)
        .await
}
```

Key changes:
- Remove `cors_headers()` function entirely
- Remove `Options` preflight handler
- Remove `.with_headers(cors)` from all responses
- Replace `auth::validate_session(&req).await` with `auth::user_id_from_gateway(&req)?`
- `/api/health` returns plain `Response::ok("ok")` without CORS headers

- [ ] **Step 3: Remove `HANKO_API_URL` compile-time dependency**

The current `auth.rs` has `const HANKO_API_URL: &str = env!("HANKO_API_URL");` — this is removed in step 1.

Update `crates/api/wrangler.toml` — add `workers_dev = false` to prevent direct public access:

```toml
name = "kartoteka-api"
main = "build/worker/shim.mjs"
compatibility_date = "2025-01-01"
account_id = "6b703fac2c4f8b359e430bf11b8dde7f"
workers_dev = false
```

- [ ] **Step 4: Verify API Worker compiles without HANKO_API_URL**

Run: `cd crates/api && API_BASE_URL="/api" cargo check`

Expected: compiles without `HANKO_API_URL` env var (since we removed `env!("HANKO_API_URL")`)

- [ ] **Step 5: Run existing tests**

Run: `API_BASE_URL="/api" cargo test --workspace`

Expected: all existing tests pass (auth tests may need updating if they exist)

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/auth.rs crates/api/src/router.rs crates/api/wrangler.toml
git commit -m "refactor(api): replace Hanko auth with X-User-Id header from Gateway"
```

---

## Task 5: MCP Server with 5 Tools

**Prerequisite:** Task 4 (API Worker uses X-User-Id header).

**Files:**
- Create: `gateway/src/mcp/tools.ts`
- Create: `gateway/src/mcp/server.ts`
- Modify: `gateway/src/index.ts`

**Docs to check:** Use `@context7` for `@modelcontextprotocol/typescript-sdk` and `@cloudflare/workers-oauth-provider` for exact integration patterns on CF Workers. Critical: check how `OAuthProvider` wraps MCP, how Streamable HTTP transport works, and whether `@cloudflare/agents` SDK provides `McpAgent` helper.

- [ ] **Step 1: Research MCP + OAuth + CF Workers integration**

Before writing code, use context7 to resolve these questions:
1. How does `McpServer` expose Streamable HTTP transport on CF Workers? (Check `@modelcontextprotocol/typescript-sdk` docs for CF Workers / Hono examples)
2. How does `OAuthProvider` wrap the MCP handler and pass `userId` from OAuth tokens to tool handlers? (Check `@cloudflare/workers-oauth-provider` AGENTS.md and examples)
3. Does `@cloudflare/agents` SDK provide `McpAgent` or `createMcpHandler` that simplifies this? If so, prefer that over manual wiring.

Document findings before proceeding to Step 2.

- [ ] **Step 2: Create `gateway/src/mcp/tools.ts`**

Define 5 tools. Each tool calls the API Worker (via service binding or DEV_API_URL fallback) and returns formatted text for LLM consumption.

```typescript
import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

async function apiCall(
  fetcher: Fetcher | undefined,
  devApiUrl: string | undefined,
  method: string,
  path: string,
  userId: string,
  body?: unknown
): Promise<Response> {
  const headers = new Headers({
    "Content-Type": "application/json",
    "X-User-Id": userId,
  });
  const init: RequestInit = { method, headers };
  if (body) init.body = JSON.stringify(body);

  if (devApiUrl) return fetch(`${devApiUrl}${path}`, init);
  return fetcher!.fetch(`https://api-worker${path}`, init);
}

export function registerTools(
  server: McpServer,
  fetcher: Fetcher | undefined,
  devApiUrl: string | undefined,
  userId: string
) {
  server.registerTool("list_lists", {
    title: "List all lists",
    description: "Returns all non-archived lists belonging to the user",
    inputSchema: z.object({}),
  }, async () => {
    const resp = await apiCall(fetcher, devApiUrl, "GET", "/api/lists", userId);
    const lists = await resp.json();
    return { content: [{ type: "text" as const, text: JSON.stringify(lists, null, 2) }] };
  });

  server.registerTool("get_list_items", {
    title: "Get list items",
    description: "Returns all items in a specific list",
    inputSchema: z.object({ list_id: z.string().describe("UUID of the list") }),
  }, async ({ list_id }) => {
    const resp = await apiCall(fetcher, devApiUrl, "GET", `/api/lists/${list_id}/items`, userId);
    if (resp.status === 404) return { content: [{ type: "text" as const, text: "List not found" }], isError: true };
    const items = await resp.json();
    return { content: [{ type: "text" as const, text: JSON.stringify(items, null, 2) }] };
  });

  server.registerTool("create_list", {
    title: "Create a new list",
    description: "Creates a new list. list_type: checklist, zakupy, pakowanie, terminarz, custom",
    inputSchema: z.object({
      name: z.string().describe("Name of the list"),
      list_type: z.enum(["checklist", "zakupy", "pakowanie", "terminarz", "custom"]).default("checklist"),
    }),
  }, async ({ name, list_type }) => {
    const resp = await apiCall(fetcher, devApiUrl, "POST", "/api/lists", userId, { name, list_type });
    const list = await resp.json();
    return { content: [{ type: "text" as const, text: JSON.stringify(list, null, 2) }] };
  });

  server.registerTool("add_item", {
    title: "Add item to list",
    description: "Adds a new item to a list",
    inputSchema: z.object({
      list_id: z.string().describe("UUID of the list"),
      title: z.string().describe("Title of the item"),
      description: z.string().optional().describe("Optional description"),
    }),
  }, async ({ list_id, title, description }) => {
    const resp = await apiCall(fetcher, devApiUrl, "POST", `/api/lists/${list_id}/items`, userId, { title, description });
    if (resp.status === 404) return { content: [{ type: "text" as const, text: "List not found" }], isError: true };
    const item = await resp.json();
    return { content: [{ type: "text" as const, text: JSON.stringify(item, null, 2) }] };
  });

  server.registerTool("toggle_item", {
    title: "Toggle item completion",
    description: "Marks an item as completed or uncompleted (calls general update endpoint with { completed })",
    inputSchema: z.object({
      list_id: z.string().describe("UUID of the list"),
      item_id: z.string().describe("UUID of the item"),
      completed: z.boolean().describe("true to mark done, false to unmark"),
    }),
  }, async ({ list_id, item_id, completed }) => {
    const resp = await apiCall(fetcher, devApiUrl, "PUT", `/api/lists/${list_id}/items/${item_id}`, userId, { completed });
    if (resp.status === 404) return { content: [{ type: "text" as const, text: "Item not found" }], isError: true };
    const item = await resp.json();
    return { content: [{ type: "text" as const, text: JSON.stringify(item, null, 2) }] };
  });
}
```

- [ ] **Step 3: Create `gateway/src/mcp/server.ts`**

Based on Step 1 research, implement the MCP server with OAuthProvider. The expected pattern (verify against context7 findings):

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { OAuthProvider } from "@cloudflare/workers-oauth-provider";
import { registerTools } from "./tools";
import type { Env } from "../types";

// The OAuthProvider wraps the MCP server as its apiHandler.
// It handles OAuth 2.1 PKCE flow (authorize, token, client registration)
// and passes authenticated user props to the API handler.
//
// The implementer MUST adapt this skeleton based on context7 research from Step 1.
// Key integration points:
// - OAuthProvider.apiHandler receives authenticated requests with user props
// - McpServer needs Streamable HTTP transport (not SSE — deprecated)
// - registerTools() is called with the authenticated userId from OAuth props
//
// If @cloudflare/agents provides McpAgent, prefer that over manual setup.

export function createMcpOAuthProvider(env: Env) {
  return new OAuthProvider({
    apiRoute: ["/mcp/"],
    apiHandler: McpApiHandler, // WorkerEntrypoint subclass — see context7
    defaultHandler: McpDefaultHandler, // consent screen + auth UI
    authorizeEndpoint: "/mcp/authorize",
    tokenEndpoint: "/mcp/oauth/token",
    clientRegistrationEndpoint: "/mcp/oauth/register",
    scopesSupported: ["read", "write"],
  });
}
```

Note: This is a skeleton. The implementer must fill in `McpApiHandler` (which creates McpServer, registers tools, and handles Streamable HTTP transport) and `McpDefaultHandler` (which renders the consent/login screen using Better Auth for authentication). Context7 research in Step 1 is critical for getting this right.

- [ ] **Step 4: Mount MCP routes in `gateway/src/index.ts`**

The MCP OAuth provider likely needs to handle its own routes outside Hono (since OAuthProvider is a CF Worker entrypoint wrapper). Two approaches:

Option A: Mount as a Hono route that delegates to OAuthProvider
```typescript
app.all("/mcp/*", async (c) => {
  const provider = createMcpOAuthProvider(c.env);
  return provider.fetch(c.req.raw, c.env, c.executionCtx);
});
```

Option B: If OAuthProvider needs to be the Worker's default export, restructure so that `index.ts` routes `/mcp/*` to OAuthProvider and everything else to Hono.

Choose based on context7 research findings.

- [ ] **Step 5: Test MCP server locally**

```bash
# Test MCP endpoint responds
curl http://localhost:8788/mcp/ -H "Accept: application/json"
```

Expected: MCP server metadata or OAuth discovery response

- [ ] **Step 6: Commit**

```bash
git add gateway/src/mcp/
git commit -m "feat(gateway): add MCP server with 5 tools and OAuth provider"
```

---

## Task 6: Update Frontend — Drop Hanko

**Files:**
- Modify: `crates/frontend/src/api/mod.rs`
- Modify: `crates/frontend/src/pages/login.rs`
- Modify: `crates/frontend/src/components/nav.rs`
- Modify: `crates/frontend/Trunk.toml` (add proxy config)
- Delete: `crates/frontend/hanko-init.js.template`
- Modify: `crates/frontend/src/app.rs` (add signup route)

- [ ] **Step 1: Rewrite `crates/frontend/src/api/mod.rs`**

Replace Hanko token auth with cookie-based auth. Key change: all requests use `credentials: "include"` instead of `Authorization: Bearer` header.

```rust
mod containers;
mod items;
mod lists;
mod tags;

pub use containers::*;
pub use items::*;
pub use lists::*;
pub use tags::*;

use gloo_net::http::{Headers, Request, RequestCredentials};

pub(crate) const API_BASE: &str = env!("API_BASE_URL");

pub(crate) fn auth_headers() -> Headers {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json");
    headers
}

pub(crate) fn get(url: &str) -> gloo_net::http::RequestBuilder {
    Request::get(url)
        .headers(auth_headers())
        .credentials(RequestCredentials::Include)
}

pub(crate) fn del(url: &str) -> gloo_net::http::RequestBuilder {
    Request::delete(url)
        .headers(auth_headers())
        .credentials(RequestCredentials::Include)
}

pub(crate) async fn post_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    Request::post(url)
        .headers(auth_headers())
        .credentials(RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn put_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    Request::put(url)
        .headers(auth_headers())
        .credentials(RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn patch_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let resp = Request::patch(url)
        .headers(auth_headers())
        .credentials(RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() >= 400 {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// Auth base URL — derived from window origin, not API_BASE path arithmetic
fn auth_base() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

/// Async session check — returns user email if logged in, None if not.
/// Used by Nav and auth guards via LocalResource.
pub async fn get_session() -> Option<SessionInfo> {
    let url = format!("{}/auth/api/get-session", auth_base());
    let resp = Request::get(&url)
        .credentials(RequestCredentials::Include)
        .send()
        .await
        .ok()?;
    if resp.status() == 200 {
        resp.json::<SessionInfo>().await.ok()
    } else {
        None
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SessionInfo {
    pub user: SessionUser,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SessionUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

pub fn logout() {
    wasm_bindgen_futures::spawn_local(async {
        let url = format!("{}/auth/api/sign-out", auth_base());
        let _ = Request::post(&url)
            .credentials(RequestCredentials::Include)
            .send()
            .await;
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/login");
        }
    });
}
```

Note: `is_logged_in()` and `get_user_email()` are replaced by `get_session()` which returns a `SessionInfo` asynchronously. Nav and protected pages use `LocalResource` to call this. Verify `gloo-net` `RequestCredentials::Include` support via context7 for `gloo-net` 0.6.

- [ ] **Step 2: Rewrite `crates/frontend/src/pages/login.rs`**

Replace `<hanko-auth>` widget with a custom login form.

```rust
use leptos::prelude::*;
use crate::api::API_BASE;

#[component]
pub fn LoginPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            let body = serde_json::json!({
                "email": email.get_untracked(),
                "password": password.get_untracked(),
            });
            let result = gloo_net::http::Request::post(
                &format!("{}/auth/api/sign-in/email", crate::api::auth_base())
            )
            .header("Content-Type", "application/json")
            .credentials(web_sys::RequestCredentials::Include)
            .body(serde_json::to_string(&body).unwrap())
            .unwrap()
            .send()
            .await;

            loading.set(false);
            match result {
                Ok(resp) if resp.ok() => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/");
                    }
                }
                Ok(resp) => {
                    error.set(Some(format!("Błąd logowania ({})", resp.status())));
                }
                Err(e) => {
                    error.set(Some(format!("Błąd sieci: {e}")));
                }
            }
        });
    };

    view! {
        <div class="flex flex-col items-center justify-center min-h-[60vh] p-4">
            <div class="card bg-base-200 border border-base-300 w-full max-w-sm">
                <div class="card-body items-center">
                    <h2 class="card-title text-2xl mb-4">"Zaloguj się"</h2>

                    {move || error.get().map(|e| view! {
                        <div class="alert alert-error mb-4">
                            <span>{e}</span>
                        </div>
                    })}

                    <form on:submit=on_submit class="w-full space-y-4">
                        <label class="input input-bordered flex items-center gap-2 w-full">
                            <input
                                type="email"
                                placeholder="Email"
                                class="grow"
                                required=true
                                on:input=move |ev| email.set(event_target_value(&ev))
                            />
                        </label>
                        <label class="input input-bordered flex items-center gap-2 w-full">
                            <input
                                type="password"
                                placeholder="Hasło"
                                class="grow"
                                required=true
                                on:input=move |ev| password.set(event_target_value(&ev))
                            />
                        </label>
                        <button
                            type="submit"
                            class="btn btn-primary w-full"
                            disabled=move || loading.get()
                        >
                            {move || if loading.get() {
                                "Logowanie..."
                            } else {
                                "Zaloguj się"
                            }}
                        </button>
                    </form>

                    <div class="divider">"lub"</div>
                    <a href="/signup" class="link link-primary">"Utwórz konto"</a>
                </div>
            </div>
        </div>
    }
}
```

- [ ] **Step 3: Update `crates/frontend/src/components/nav.rs`**

Replace synchronous `is_logged_in()` / `get_user_email()` with async `get_session()` via `LocalResource`:

```rust
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use crate::api;

#[component]
pub fn Nav() -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);

    // Async session check
    let session = LocalResource::new(|| async {
        SendWrapper::new(api::get_session().await)
    });

    let on_logout = move |_| {
        api::logout();
    };

    view! {
        <nav class="navbar bg-base-200 border-b border-base-300 px-4">
            <div class="navbar-start">
                <a href="/" style="text-decoration: none;">
                    <span class="text-xl font-bold text-primary">"Kartoteka"</span>
                </a>
            </div>
            <div class="navbar-end">
                <Suspense fallback=|| ()>
                    {move || {
                        let sess = session.get().map(|s| (*s).clone()).flatten();
                        if let Some(info) = sess {
                            let email_display = if info.user.email.is_empty() {
                                "Konto".to_string()
                            } else {
                                info.user.email.clone()
                            };
                            view! {
                                <a href="/today" class="btn btn-ghost btn-sm">"Dziś"</a>
                                <a href="/calendar" class="btn btn-ghost btn-sm">"Kalendarz"</a>
                                <div class="relative">
                                    <button class="btn btn-ghost btn-sm"
                                        on:click=move |_| set_menu_open.update(|v| *v = !*v)>
                                        {email_display}
                                    </button>
                                    <ul class="menu bg-base-200 rounded-box border border-base-300 shadow-lg z-50 min-w-40 absolute right-0 top-full mt-1"
                                        style:display=move || if menu_open.get() { "block" } else { "none" }>
                                        <li><a href="/tags">"Tagi"</a></li>
                                        <li><a href="/settings">"Ustawienia"</a></li>
                                        <li><button type="button" on:click=on_logout>"Wyloguj"</button></li>
                                    </ul>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <a href="/login" class="btn btn-primary btn-sm">"Zaloguj"</a>
                            }.into_any()
                        }
                    }}
                </Suspense>
            </div>
        </nav>
    }
}
```

- [ ] **Step 4: Create signup page**

Create `crates/frontend/src/pages/signup.rs` — similar to login but calls `/auth/api/sign-up/email` with email, password, name fields. Use `auth_base()` for URL construction (not `API_BASE`).

- [ ] **Step 5: Add signup route to `app.rs`**

```rust
// Add import
use crate::pages::signup::SignupPage;

// Add route
<Route path=path!("/signup") view=SignupPage/>
```

- [ ] **Step 6: Update `crates/frontend/Trunk.toml`**

Add proxy config for both `/api` and `/auth` routes to Gateway:

```toml
[build]
target = "index.html"
dist = "dist"

[watch]
watch = ["src", "style/input.css", "index.html"]

[[hooks]]
stage = "pre_build"
command = "npx"
command_arguments = ["@tailwindcss/cli", "-i", "style/input.css", "-o", "style/main.css"]

[[proxy]]
backend = "http://localhost:8788"
rewrite = "/api"

[[proxy]]
backend = "http://localhost:8788"
rewrite = "/auth"
```

- [ ] **Step 7: Delete `crates/frontend/hanko-init.js.template`**

```bash
rm crates/frontend/hanko-init.js.template
```

Also remove any `<script>` tag in `crates/frontend/index.html` that loads `hanko-init.js`.

- [ ] **Step 8: Verify frontend compiles**

Run: `cd crates/frontend && API_BASE_URL="/api" cargo check --target wasm32-unknown-unknown`

Expected: compiles without errors

- [ ] **Step 9: Commit**

```bash
git add crates/frontend/src/api/mod.rs crates/frontend/src/pages/login.rs crates/frontend/src/pages/signup.rs crates/frontend/src/pages/mod.rs crates/frontend/src/app.rs crates/frontend/src/components/nav.rs crates/frontend/Trunk.toml
git rm crates/frontend/hanko-init.js.template
git commit -m "refactor(frontend): replace Hanko with Better Auth cookie-based login"
```

---

## Task 7: Update Justfile + Dev Workflow

**Files:**
- Modify: `justfile`
- Modify: `.gitignore` (if needed)

- [ ] **Step 1: Update `justfile`**

Key changes:
- Remove all `_gen-hanko` / `_gen-hanko-prod` recipes
- Remove `HANKO_API_URL` from all commands
- Replace `mcp` references with `gateway`
- Add `dev-gateway` recipe
- Update `dev` to run gateway + API + frontend
- Update `deploy` to include gateway

```just
# Kartoteka — task runner
set dotenv-load

export CLOUDFLARE_ACCOUNT_ID := env("CLOUDFLARE_ACCOUNT_ID", "")

default:
    @just --list

# === SETUP ===

setup:
    cargo install trunk worker-build
    rustup target add wasm32-unknown-unknown
    cd gateway && npm install
    cd crates/frontend && npm install

# === DEV ===

# Uruchom API worker lokalnie
dev-api:
    cd crates/api && npx wrangler dev --env local --local --port 8787

# Uruchom Gateway worker lokalnie
dev-gateway:
    cd gateway && npx wrangler dev --env local --local --port 8788

# Uruchom frontend (proxy config in Trunk.toml)
dev-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="/api" trunk serve

# Uruchom API + Gateway + frontend
dev:
    just dev-api & just dev-gateway & just dev-frontend & wait

# === BUILD ===

build: build-api build-frontend build-gateway

build-api:
    cd crates/api && worker-build --release

build-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${API_BASE_URL}" trunk build --release

build-gateway:
    cd gateway && npx wrangler deploy --dry-run

check:
    API_BASE_URL="/api" cargo check --workspace

# === MIGRACJE ===

migrate-create NAME:
    cd crates/api && npx wrangler d1 migrations create kartoteka-db {{NAME}}

migrate-local:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-api-local --env local --local

migrate-dev:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-dev --env dev --remote

migrate-prod:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --env="" --remote

migrate-remote: migrate-prod

# Gateway auth DB migrations
migrate-gateway-local:
    cd gateway && npx wrangler d1 migrations apply kartoteka-gateway-local --env local --local

migrate-gateway-dev:
    cd gateway && npx wrangler d1 migrations apply kartoteka-auth-dev --env dev --remote

migrate-gateway-prod:
    cd gateway && npx wrangler d1 migrations apply kartoteka-auth --remote

# === DEPLOY ===

deploy: deploy-migrate migrate-gateway-prod deploy-api deploy-gateway deploy-frontend

deploy-dev: migrate-dev deploy-api-dev deploy-frontend-dev

deploy-migrate:
    cd crates/api && npx wrangler d1 migrations apply kartoteka-db --remote

deploy-api:
    cd crates/api && npx wrangler deploy

deploy-api-dev:
    cd crates/api && npx wrangler deploy --env dev

deploy-gateway:
    cd gateway && npx wrangler deploy

deploy-frontend:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="${API_BASE_URL}" trunk build --release
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=main --commit-dirty=true

deploy-frontend-dev:
    cd crates/frontend && npm install
    cd crates/frontend && API_BASE_URL="https://kartoteka-gateway.jpalczewski.workers.dev/api" trunk build --release
    npx wrangler pages deploy crates/frontend/dist --project-name=kartoteka --branch=dev --commit-dirty=true

# === QUALITY ===

lint:
    API_BASE_URL="/api" cargo clippy --workspace -- -D warnings
    cargo fmt --check --all

fmt:
    cargo fmt --all

audit:
    cargo deny check

machete:
    cargo machete

test:
    API_BASE_URL="/api" cargo test --workspace

ci: fmt lint audit machete test
```

- [ ] **Step 2: Update `.env.example`** (if it exists, or document required vars)

New required env vars:
```
CLOUDFLARE_ACCOUNT_ID=...
API_BASE_URL=...
BETTER_AUTH_SECRET=...
BETTER_AUTH_URL=...
GITHUB_CLIENT_ID=...      # optional
GITHUB_CLIENT_SECRET=...   # optional
```

Removed:
```
HANKO_API_URL  # no longer needed
DEV_AUTH_TOKEN # no longer needed
```

- [ ] **Step 3: Remove `mcp/` directory**

```bash
rm -rf mcp/
```

- [ ] **Step 4: Verify full dev workflow**

```bash
just dev
```

Expected: all three services start (API on 8787, Gateway on 8788, frontend on trunk default port)

- [ ] **Step 5: Commit**

```bash
git rm -r mcp/
git add justfile
git commit -m "chore: update justfile for Gateway, remove old mcp/ scaffold and Hanko references"
```

---

## Task 8: Update CLAUDE.md + CI

**Files:**
- Modify: `CLAUDE.md`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Update `CLAUDE.md`**

Key changes:
- Remove all Hanko references (hanko-init.js, HANKO_API_URL, bridge JS/WASM)
- Add Gateway Worker section
- Update Auth section to describe Better Auth
- Update Env vars section
- Update "pliki do nie commitowania" section
- Update architecture description

- [ ] **Step 2: Remove `HANKO_API_URL` from `.github/workflows/ci.yml`**

Line 11 of `ci.yml` currently sets `HANKO_API_URL: "https://placeholder.hanko.io"` in the env block. Remove this line — the API Worker no longer requires it at compile time.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md .github/
git commit -m "docs: update CLAUDE.md and CI for Gateway + Better Auth"
```

---

## Task 9: End-to-End Smoke Test

- [ ] **Step 1: Start all services locally**

```bash
just dev
```

- [ ] **Step 2: Test signup via frontend**

Open browser, go to `/signup`, create account with email+password.
Expected: redirects to `/` after successful signup.

- [ ] **Step 3: Test login via frontend**

Log out, go to `/login`, sign in with the account.
Expected: redirects to `/` after successful login.

- [ ] **Step 4: Test API calls through Gateway**

With the session active, verify that list operations work:
- Create a list
- Add an item
- Toggle the item

Expected: all operations succeed (data visible in UI).

- [ ] **Step 5: Test MCP tools locally**

Use MCP Inspector or curl to test MCP tools against the Gateway:
```bash
# List tools
curl http://localhost:8788/mcp/ -H "Accept: application/json"
```

- [ ] **Step 6: Test unauthenticated access is blocked**

```bash
curl http://localhost:8788/api/lists
```
Expected: 401 (unless DEV_AUTH_USER_ID is set in local env)

- [ ] **Step 7: Commit any fixes discovered during testing**

```bash
git add -A
git commit -m "fix: address issues found during e2e smoke testing"
```
