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
