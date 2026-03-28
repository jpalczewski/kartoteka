import { Hono } from "hono";
import type { Env, Variables } from "./types";

const proxy = new Hono<{ Bindings: Env; Variables: Variables }>();

proxy.all("/*", async (c) => {
  const userId = c.get("userId");
  const headers = new Headers(c.req.raw.headers);
  headers.set("X-User-Id", userId);

  if (c.env.DEV_API_URL) {
    // Local dev: rewrite URL to DEV_API_URL
    const devUrl = new URL(c.req.url);
    const apiBase = new URL(c.env.DEV_API_URL);
    devUrl.hostname = apiBase.hostname;
    devUrl.port = apiBase.port;
    devUrl.protocol = apiBase.protocol;
    return fetch(
      new Request(devUrl.toString(), {
        method: c.req.method,
        headers,
        body: c.req.raw.body,
      })
    );
  }

  return c.env.API_WORKER.fetch(
    new Request(c.req.url, {
      method: c.req.method,
      headers,
      body: c.req.raw.body,
    })
  );
});

export { proxy };
