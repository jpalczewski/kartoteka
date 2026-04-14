import { createMiddleware } from "hono/factory";
import type { Env, Variables } from "./types";
import { getAuth } from "./auth";
import { log } from "./logger";

export const authMiddleware = createMiddleware<{ Bindings: Env; Variables: Variables }>(
  async (c, next) => {
    if (c.env.DEV_AUTH_USER_ID) {
      c.set("userId", c.env.DEV_AUTH_USER_ID);
      c.set("userEmail", "dev@local");
      return next();
    }

    const auth = getAuth(c.env);
    const session = await auth.api.getSession({ headers: c.req.raw.headers });

    if (!session?.user?.id) {
      log("WARN", "auth failed", {
        request_id: c.get("requestId") ?? "",
        path: new URL(c.req.url).pathname,
      });
      return c.json({ error: "Unauthorized" }, 401);
    }

    c.set("userId", session.user.id);
    c.set("userEmail", session.user.email ?? "");
    log("INFO", "auth success", {
      request_id: c.get("requestId") ?? "",
      user_id: session.user.id,
    });
    return next();
  }
);
