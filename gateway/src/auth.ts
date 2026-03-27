import { betterAuth } from "better-auth";
import type { Env } from "./types";

export function createAuth(env: Env) {
  return betterAuth({
    database: env.AUTH_DB,
    secret: env.BETTER_AUTH_SECRET,
    baseURL: env.BETTER_AUTH_URL,
    basePath: "/auth",
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
