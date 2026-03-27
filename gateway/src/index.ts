import { Hono } from "hono";
import { cors } from "hono/cors";
import { getMigrations } from "better-auth/db/migration";
import type { Env, Variables } from "./types";
import { createAuth } from "./auth";

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

// TODO: restrict CORS origins to frontend domain in production
app.use("/api/*", cors());
app.use("/auth/*", cors());

app.get("/health", (c) => c.text("ok"));

// Run Better Auth programmatic migrations for D1
// Call manually after deploy to set up / update auth tables
app.post("/migrate", async (c) => {
  try {
    const auth = createAuth(c.env);
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
  const auth = createAuth(c.env);
  return auth.handler(c.req.raw);
});

export default app;
