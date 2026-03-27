export interface Env {
  AUTH_DB: D1Database;
  OAUTH_KV: KVNamespace;
  API_WORKER: Fetcher;
  BETTER_AUTH_SECRET: string;
  BETTER_AUTH_URL: string;
  GITHUB_CLIENT_ID?: string;
  GITHUB_CLIENT_SECRET?: string;
  DEV_AUTH_USER_ID?: string;
  DEV_API_URL?: string;
}

export interface Variables {
  userId: string;
}
