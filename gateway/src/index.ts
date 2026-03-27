import { Hono } from "hono";
import { cors } from "hono/cors";
import type { Env, Variables } from "./types";

const app = new Hono<{ Bindings: Env; Variables: Variables }>();

app.use("/api/*", cors());
app.use("/auth/*", cors());

app.get("/health", (c) => c.text("ok"));

export default app;
