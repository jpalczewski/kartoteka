import { Hono } from "hono";
import { cors } from "hono/cors";
import type { Env, Variables } from "./types";
import { createAuth } from "./auth";

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

// TODO: restrict CORS origins to frontend domain in production
app.use("/api/*", cors());
app.use("/auth/*", cors());

app.get("/health", (c) => c.text("ok"));

app.all("/auth/*", async (c) => {
  const auth = createAuth(c.env);
  return auth.handler(c.req.raw);
});

export default app;
