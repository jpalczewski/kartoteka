import { betterAuth } from "better-auth";
import type { Env } from "./types";

const authCache = new WeakMap<object, ReturnType<typeof createAuth>>();

export function getAuth(env: Env): ReturnType<typeof createAuth> {
  if (!authCache.has(env)) {
    authCache.set(env, createAuth(env));
  }
  return authCache.get(env)!;
}

export function createAuth(env: Env) {
  return betterAuth({
    database: env.AUTH_DB,
    secret: env.BETTER_AUTH_SECRET,
    baseURL: env.BETTER_AUTH_URL,
    basePath: "/auth/api",
    advanced: {
      crossSubDomainCookies: { enabled: false },
      cookies: {
        session_token: {
          attributes: { sameSite: "none", secure: true },
        },
      },
    },
    trustedOrigins: async (request) => {
      const base = [env.BETTER_AUTH_URL];
      if (!request) return base;
      const origin = request.headers.get("origin") ?? "";
      // Trust any localhost / 127.0.0.1 origin in local dev
      if (
        origin.startsWith("http://localhost:") ||
        origin.startsWith("http://127.0.0.1:")
      ) {
        return [origin];
      }
      return env.TRUSTED_ORIGINS
        ? env.TRUSTED_ORIGINS.split(",").map((o) => o.trim())
        : base;
    },
    emailAndPassword: {
      enabled: true,
    },
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
