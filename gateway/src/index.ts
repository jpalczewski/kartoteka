import { Hono } from "hono";
import { cors } from "hono/cors";
import { getMigrations } from "better-auth/db/migration";
import { OAuthProvider } from "@cloudflare/workers-oauth-provider";
import { nanoid } from "nanoid";
import type { Env, Variables } from "./types";
import { getAuth } from "./auth";
import { authMiddleware } from "./middleware";
import { proxy } from "./proxy";
import { McpApiHandler } from "./mcp/server";
import { log } from "./logger";

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

app.use("*", async (c, next) => {
  const corsMiddleware = cors({
    origin: allowedOrigins(c.env),
    credentials: true,
  });
  return corsMiddleware(c, next);
});

app.use("*", async (c, next) => {
  const requestId = c.req.header("x-request-id") ?? nanoid();
  c.set("requestId", requestId);
  c.header("X-Request-Id", requestId);

  const start = Date.now();
  await next();

  log("INFO", "request completed", {
    request_id: requestId,
    method: c.req.method,
    path: new URL(c.req.url).pathname,
    status: c.res.status,
    duration_ms: Date.now() - start,
  });
});

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

// Intercept signup to enforce registration mode and validate invite codes
app.post("/auth/api/sign-up/email", async (c) => {
  // Skip in local dev (DEV_AUTH_USER_ID bypasses auth entirely)
  if (!c.env.DEV_AUTH_USER_ID) {
    const body = await c.req.json<Record<string, string>>();

    // Call the Rust API to get the current registration mode
    const apiBase = c.env.DEV_API_URL ?? "http://internal";
    const modeRes = await (c.env.DEV_API_URL
      ? fetch(`${apiBase}/api/public/registration-mode`)
      : c.env.API_WORKER.fetch(new Request(`${apiBase}/api/public/registration-mode`)));

    if (!modeRes.ok) {
      return c.json({ error: "Failed to check registration mode" }, 500);
    }

    const { mode } = await modeRes.json<{ mode: string }>();

    if (mode === "closed") {
      return c.json({ error: "Registration is closed" }, 403);
    }

    if (mode === "invite") {
      const inviteCode = body.inviteCode;
      if (!inviteCode) {
        return c.json({ error: "Invite code required" }, 403);
      }

      const validateRes = await (c.env.DEV_API_URL
        ? fetch(`${apiBase}/api/public/validate-invite`, {
            method: "POST",
            body: JSON.stringify({ code: inviteCode, email: body.email }),
            headers: { "Content-Type": "application/json" },
          })
        : c.env.API_WORKER.fetch(
            new Request(`${apiBase}/api/public/validate-invite`, {
              method: "POST",
              body: JSON.stringify({ code: inviteCode, email: body.email }),
              headers: { "Content-Type": "application/json" },
            })
          ));

      if (!validateRes.ok) {
        return c.json({ error: "Failed to validate invite code" }, 500);
      }

      const { valid, error } = await validateRes.json<{ valid: boolean; error?: string }>();
      if (!valid) {
        return c.json({ error: error ?? "Invalid invite code" }, 403);
      }
    }

    // Strip inviteCode before forwarding to Better Auth
    const { inviteCode: _removed, ...authBody } = body;
    const auth = getAuth(c.env);
    return auth.handler(
      new Request(c.req.raw.url, {
        method: "POST",
        body: JSON.stringify(authBody),
        headers: c.req.raw.headers,
      })
    );
  }

  // Dev mode: forward directly
  const auth = getAuth(c.env);
  return auth.handler(c.req.raw);
});

app.all("/auth/*", async (c) => {
  const auth = getAuth(c.env);
  return auth.handler(c.req.raw);
});

// Generate a one-time consent token (called by frontend via fetch)
app.post("/oauth/consent-token", async (c) => {
  const auth = getAuth(c.env);
  const session = await auth.api.getSession({ headers: c.req.raw.headers });
  if (!session) return c.json({ error: "Unauthorized" }, 401);

  const token = crypto.randomUUID();
  await c.env.OAUTH_KV.put(
    `consent:${token}`,
    JSON.stringify({ userId: session.user.id }),
    { expirationTtl: 300 }
  );
  return c.json({ consent_token: token });
});

// MCP OAuth authorize — consent_token flow (no HTML rendered)
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

  const url = new URL(request.url);
  const consentToken = url.searchParams.get("consent_token");

  if (consentToken) {
    // Returning from frontend consent — validate token and complete authorization
    const stored = await env.OAUTH_KV.get(`consent:${consentToken}`, { type: "json" }) as {
      userId: string;
    } | null;
    if (!stored) {
      return c.json({ error: "Invalid or expired consent token" }, 400);
    }
    await env.OAUTH_KV.delete(`consent:${consentToken}`);

    const { redirectTo } = await env.OAUTH_PROVIDER.completeAuthorization({
      request: oauthReqInfo,
      userId: stored.userId,
      metadata: { label: "Claude MCP access" },
      scope: oauthReqInfo.scope ?? [],
      props: { userId: stored.userId },
    });
    return Response.redirect(redirectTo, 302);
  }

  // No consent_token — redirect to frontend consent page (handles login + consent)
  const frontendOrigin = allowedOrigins(env)[0];
  const consentUrl = `${frontendOrigin}/oauth/consent?${url.searchParams.toString()}`;
  return Response.redirect(consentUrl, 302);
});

// API routes — require auth, then proxy to Rust API Worker
app.use("/api/*", authMiddleware);
app.route("/api", proxy);

export { McpApiHandler };

export default new OAuthProvider({
  apiRoute: "/mcp",
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
