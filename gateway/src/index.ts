import { Hono } from "hono";
import { cors } from "hono/cors";
import { getMigrations } from "better-auth/db/migration";
import type { Env, Variables } from "./types";
import { getAuth } from "./auth";

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

app.use("/api/*", (c, next) =>
  cors({ origin: c.env.BETTER_AUTH_URL, credentials: true })(c, next)
);
app.use("/auth/*", (c, next) =>
  cors({ origin: c.env.BETTER_AUTH_URL, credentials: true })(c, next)
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

app.all("/auth/*", async (c) => {
  const auth = getAuth(c.env);
  return auth.handler(c.req.raw);
});

export default app;
