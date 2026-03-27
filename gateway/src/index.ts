import { Hono } from "hono";
import { cors } from "hono/cors";
import { getMigrations } from "better-auth/db/migration";
import { OAuthProvider } from "@cloudflare/workers-oauth-provider";
import type { Env, Variables } from "./types";
import { getAuth } from "./auth";
import { authMiddleware } from "./middleware";
import { proxy } from "./proxy";
import { McpApiHandler } from "./mcp/server";

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

const allowedOrigins = (env: Env) =>
  env.TRUSTED_ORIGINS ? env.TRUSTED_ORIGINS.split(",") : [env.BETTER_AUTH_URL];

app.use("/api/*", (c, next) =>
  cors({ origin: allowedOrigins(c.env), credentials: true })(c, next)
);
app.use("/auth/*", (c, next) =>
  cors({ origin: allowedOrigins(c.env), credentials: true })(c, next)
);

app.get("/health", (c) => c.text("ok"));

// Run Better Auth programmatic migrations for D1
// Call manually after deploy to set up / update auth tables
app.post("/migrate", async (c) => {
  const secret = c.req.header("x-migrate-secret");
  if (!secret || secret !== c.env.MIGRATE_SECRET) {
    return c.json({ error: "Unauthorized" }, 401);
  }
  try {
    const auth = getAuth(c.env);
    const { toBeCreated, toBeAdded, runMigrations } = await getMigrations(auth.options);
    if (toBeCreated.length === 0 && toBeAdded.length === 0) {
      return c.json({ message: "No migrations needed" });
    }
    await runMigrations();
    return c.json({
      message: "Migrations completed successfully",
      created: toBeCreated.map((t) => t.table),
      added: toBeAdded.map((t) => t.table),
    });
  } catch (error) {
    return c.json({ error: error instanceof Error ? error.message : "Migration failed" }, 500);
  }
});

// In local dev, return a fake session so the frontend doesn't force login
app.get("/auth/api/get-session", async (c, next) => {
  if (!c.env.DEV_AUTH_USER_ID) return next();
  return c.json({
    session: { id: "dev-session", userId: c.env.DEV_AUTH_USER_ID, expiresAt: "2099-01-01T00:00:00.000Z" },
    user: { id: c.env.DEV_AUTH_USER_ID, email: "dev@local", name: "Dev User", emailVerified: true },
  });
});

app.all("/auth/*", async (c) => {
  const auth = getAuth(c.env);
  return auth.handler(c.req.raw);
});

// MCP OAuth consent screen — must NOT be under /mcp/ (OAuthProvider treats that as API route)
// GET: show login form (if not authenticated) or consent screen (if authenticated)
// POST: complete the OAuth authorization flow
app.all("/oauth/authorize", async (c) => {
  const env = c.env;
  const request = c.req.raw;

  // Parse the OAuth request parameters
  let oauthReqInfo;
  try {
    oauthReqInfo = await env.OAUTH_PROVIDER.parseAuthRequest(request);
  } catch {
    return c.html(
      `<!DOCTYPE html><html><body><h1>Invalid authorization request</h1></body></html>`,
      400
    );
  }

  // Check for an existing Better Auth session
  const auth = getAuth(env);
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session) {
    // No session — show a combined login + consent form
    const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Kartoteka — Grant Access to Claude</title>
  <style>
    body { font-family: sans-serif; max-width: 400px; margin: 80px auto; padding: 0 16px; }
    h1 { font-size: 1.4rem; }
    label { display: block; margin-top: 12px; font-size: 0.9rem; }
    input { width: 100%; padding: 8px; margin-top: 4px; box-sizing: border-box; }
    button { margin-top: 20px; width: 100%; padding: 10px; background: #6366f1; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 1rem; }
  </style>
</head>
<body>
  <h1>Grant Kartoteka access to Claude</h1>
  <p>Sign in to your Kartoteka account to authorize Claude to access your lists.</p>
  <form method="POST" action="/auth/api/sign-in/email">
    <input type="hidden" name="callbackURL" value="/oauth/authorize?${new URL(request.url).searchParams.toString()}">
    <label>Email<input type="email" name="email" required></label>
    <label>Password<input type="password" name="password" required></label>
    <button type="submit">Sign in &amp; Authorize</button>
  </form>
</body>
</html>`;
    return c.html(html);
  }

  if (request.method === "POST") {
    // CSRF protection: verify Origin matches our host
    const origin = request.headers.get("origin");
    const expectedOrigin = new URL(request.url).origin;
    if (!origin || origin !== expectedOrigin) {
      return c.json({ error: "Forbidden" }, 403);
    }
    // User approved — complete the authorization
    const { redirectTo } = await env.OAUTH_PROVIDER.completeAuthorization({
      request: oauthReqInfo,
      userId: session.user.id,
      metadata: { label: "Claude MCP access" },
      scope: oauthReqInfo.scope ?? [],
      props: { userId: session.user.id },
    });
    return Response.redirect(redirectTo, 302);
  }

  // GET with valid session — show consent screen
  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Kartoteka — Grant Access to Claude</title>
  <style>
    body { font-family: sans-serif; max-width: 400px; margin: 80px auto; padding: 0 16px; }
    h1 { font-size: 1.4rem; }
    p { color: #555; }
    button { margin-top: 20px; width: 100%; padding: 10px; background: #6366f1; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 1rem; }
  </style>
</head>
<body>
  <h1>Grant Kartoteka access to Claude</h1>
  <p>Signed in as <strong>${escapeHtml(session.user.email)}</strong></p>
  <p>Claude is requesting access to your Kartoteka lists.</p>
  <form method="POST">
    <button type="submit">Approve</button>
  </form>
</body>
</html>`;
  return c.html(html);
});

// API routes — require auth, then proxy to Rust API Worker
app.use("/api/*", authMiddleware);
app.route("/api", proxy);

export { McpApiHandler };

export default new OAuthProvider({
  apiRoute: "/mcp/",
  apiHandler: McpApiHandler,
  defaultHandler: {
    fetch: (request: Request, env: unknown, ctx: ExecutionContext) =>
      app.fetch(request, env as Env, ctx),
  },
  authorizeEndpoint: "/oauth/authorize",
  tokenEndpoint: "/mcp/oauth/token",
  clientRegistrationEndpoint: "/mcp/oauth/register",
  scopesSupported: ["read", "write"],
  accessTokenTTL: 3600,
});
