import { createMiddleware } from "hono/factory";
import type { Env, Variables } from "./types";
import { getAuth } from "./auth";

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
      return c.json({ error: "Unauthorized" }, 401);
    }

    c.set("userId", session.user.id);
    c.set("userEmail", session.user.email ?? "");
    return next();
  }
);
