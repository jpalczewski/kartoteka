# VPS Bootstrap Guide

This directory contains templates for deploying kartoteka on a VPS using Docker Compose and Caddy.

## Prerequisites

- Docker 24+ with docker-compose support
- DNS A records configured for:
  - `kartoteka.example.com` (apex)
  - `dev.kartoteka.example.com` (dev environment)
  - `pr.kartoteka.example.com` (preview/PR environment)
- GitHub repository secrets configured on the repo:
  - `SSH_HOST` — VPS hostname or IP
  - `SSH_USER` — deploy user (see step 1)
  - `SSH_PRIVATE_KEY` — SSH private key for the deploy user
- A user with docker group access (created in step 1)

## Bootstrap Steps

### 1. Create deploy user

```bash
sudo useradd -m -G docker -s /bin/bash deploy
```

This creates a user with Docker access, allowing automated deploys without sudo.

### 2. Create directories

```bash
sudo mkdir -p /srv/kartoteka/data
sudo chown -R deploy:deploy /srv/kartoteka
```

The `data/` subdirectory will hold SQLite database files.

### 3. Copy deploy files

Copy the templates from `deploy/` in this repo to `/srv/kartoteka/` on the VPS (via scp or your preferred method):

```bash
scp deploy/compose.yml deploy/Caddyfile deploy/.env.example deploy/.env.app.example \
    deploy@your-vps:/srv/kartoteka/
```

Avoid cloning the full repo on the VPS — it creates a stale checkout that rots over time.

### 4. Create per-environment env files

Use `.env.app.example` as the template (not `.env.example` — that one is for Caddy only and must not contain app secrets).

```bash
cd /srv/kartoteka
cp .env.app.example .env.prod
cp .env.app.example .env.dev
cp .env.app.example .env.preview
```

Edit each file. The only values that differ per-env are `PUBLIC_BASE_URL` and `OAUTH_SIGNING_SECRET` (generate a separate secret per env with `openssl rand -base64 32`):

**`.env.prod`**:
```bash
DATABASE_URL=sqlite:///app/kartoteka.db?mode=rwc
OAUTH_SIGNING_SECRET=<32-char-random>
PUBLIC_BASE_URL=https://kartoteka.example.com
APP_ENV=production
BIND_ADDR=0.0.0.0:3000
RUST_LOG=info
```

**`.env.dev`**:
```bash
DATABASE_URL=sqlite:///app/kartoteka.db?mode=rwc
OAUTH_SIGNING_SECRET=<different-32-char-random>
PUBLIC_BASE_URL=https://dev.kartoteka.example.com
APP_ENV=production
BIND_ADDR=0.0.0.0:3000
RUST_LOG=debug
```

**`.env.preview`**:
```bash
DATABASE_URL=sqlite:///app/kartoteka.db?mode=rwc
OAUTH_SIGNING_SECRET=<different-32-char-random>
PUBLIC_BASE_URL=https://pr.kartoteka.example.com
APP_ENV=production
BIND_ADDR=0.0.0.0:3000
RUST_LOG=info
```

> `APP_ENV=production` on all envs — the server uses this to enable structured JSON logs. Use `RUST_LOG=debug` on dev/preview for verbose output instead.

Set restrictive permissions:

```bash
chmod 600 .env.prod .env.dev .env.preview
```

### 5. Create root .env for Caddy

```bash
cd /srv/kartoteka
cp .env.example .env
# Edit: replace kartoteka.example.com with your actual domain
```

This file is read only by the Caddy service (`env_file: .env` in compose.yml). Keep it separate from per-app env files — it must not contain app secrets.

### 6. Create empty SQLite databases

```bash
cd /srv/kartoteka
touch data/{prod,dev,preview}.db

# Ensure the app user (UID 1001 in the image) can read/write
sudo chown 1001:1001 data/*.db
chmod 660 data/*.db
```

The Docker image runs as non-root user UID 1001. Migrations run automatically on startup.

### 7. Authorize SSH key

On the VPS as the deploy user:

```bash
mkdir -p ~/.ssh
touch ~/.ssh/authorized_keys
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys
```

Add the public key from your CI/CD pipeline to `~/.ssh/authorized_keys`.

### 8. (Optional) Add KARTOTEKA_DOMAIN to compose.yml

If not using a root `.env` file, add the domain directly to the caddy service:

```yaml
  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    environment:
      KARTOTEKA_DOMAIN: kartoteka.example.com
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    networks: [app]
```

### 9. Start Caddy

```bash
cd /srv/kartoteka
docker compose pull caddy
docker compose up -d caddy
```

This starts only the Caddy reverse proxy. Apps will be deployed separately via CI/CD.

### 10. Validate TLS and reverse proxy

```bash
curl -v https://kartoteka.example.com
```

You should see:
- Valid TLS certificate (issued by Let's Encrypt)
- HTTP 502 (Bad Gateway) — Caddy can reach the reverse_proxy endpoint, but the app isn't running yet

Once the `prod` service is running, you'll get 200 OK from the health endpoint.

## Deploying Applications

Deployment is handled by GitHub Actions workflows:
- **Production**: `docker-prod.yml` (triggered by tag push, e.g., `git tag v0.4.1 && git push --tags`)
- **Development**: `docker-dev.yml` (triggered by pushes to `develop` branch)
- **Preview**: `docker-preview.yml` (triggered by pushes to specific PR branches)

Each workflow:
1. Builds the Docker image
2. Pushes to `ghcr.io/jpalczewski/kartoteka-a1:TAG`
3. SSH into VPS and runs `docker compose pull` + `docker compose up -d SERVICE`

## Rollback Procedure

To roll back production to a previous version:

1. In GitHub, navigate to **Actions** → **docker-prod** workflow
2. Click **Run workflow** (workflow_dispatch)
3. Enter the tag to deploy (e.g., `v0.4.0`)
4. Click **Run workflow**

The workflow will deploy the specified image tag to the prod service.

Alternatively, manually on the VPS:

```bash
cd /srv/kartoteka
docker compose pull prod  # pulls latest
# or specify a tag:
KARTOTEKA_PROD_TAG=v0.4.0 docker compose pull prod
docker compose up -d prod
```

## ghcr.io Package Visibility

For free public image pulls from the VPS without docker login:

1. Go to GitHub repo → **Packages** → `kartoteka-a1` package
2. Click **Package settings**
3. Set **Visibility** to **Public**
4. Save

Note: The `:develop` and `:preview` tags are overwritten on each build to the respective branches. Tag production builds with semantic versions (e.g., `v0.4.1`).

## Environment Variables Reference

All variables go into the per-environment file (`.env.prod`, `.env.dev`, `.env.preview`).
See `.env.app.example` for a ready-to-copy template.

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | yes | — | SQLite connection string. Format: `sqlite:///absolute/path/kartoteka.db?mode=rwc`. Use a separate file per environment. |
| `OAUTH_SIGNING_SECRET` | yes | — | HMAC-HS256 key used to sign OAuth access tokens and personal access tokens. **Minimum 32 characters, cryptographically random.** Generate with `openssl rand -base64 32`. Use a **distinct secret per environment** — a shared key means a dev token works in prod. |
| `PUBLIC_BASE_URL` | yes | `http://localhost:3000` | Full public origin (scheme + host, no trailing slash), e.g. `https://kartoteka.example.com`. Used in OAuth metadata (`issuer`, `authorization_endpoint`), redirect URI validation, and OAuth redirect construction. Also triggers `Secure` session cookies automatically when it starts with `https://`. |
| `APP_ENV` | no | _(unset)_ | Set to `production` on all deployed environments. Controls: (1) structured JSON log output; (2) `Secure` flag on session cookies so they are only sent over HTTPS. |
| `BIND_ADDR` | no | from Leptos config (`0.0.0.0:3000`) | TCP address the server listens on inside the container. Caddy forwards external traffic to this. |
| `RUST_LOG` | no | `kartoteka_server=debug,...` | Log filter in `tracing-subscriber` `EnvFilter` format. Use `info` for prod, `debug` for dev/preview. |

### Security: `OAUTH_SIGNING_SECRET`

The secret is the HMAC key for:
- **MCP OAuth access tokens** — short-lived (~1 h TTL, `scope=mcp`)
- **Personal access tokens** — user-created, default 90-day TTL, max 365 days (`scope=full`)

If it leaks, all tokens signed with it must be considered compromised. To rotate:

1. Generate a new secret: `openssl rand -base64 32`
2. Update `.env.prod` on the VPS
3. Restart: `docker compose up -d prod`

After restart all existing tokens are invalid — users must re-authenticate MCP clients and recreate personal access tokens.

### Security: `APP_ENV` and session cookies

Session cookies carry the authenticated user identity and the OAuth consent CSRF token.

When `APP_ENV=production` **or** `PUBLIC_BASE_URL` starts with `https://`, cookies are issued with:

```
Set-Cookie: id=...; Secure; HttpOnly; SameSite=Lax
```

Without `Secure`, browsers send the cookie over plain HTTP, enabling session hijacking on hostile networks.

For local dev (`http://localhost`) leave `APP_ENV` unset — `Secure` cookies do not work over HTTP.

## Troubleshooting

### Caddy won't start
- Check `/var/lib/docker/volumes/kartoteka_caddy_data/_data/` for logs
- Validate Caddyfile syntax: `caddy validate --config Caddyfile`
- Check environment variables are set: `docker compose config | grep KARTOTEKA_DOMAIN`

### App won't start
- Check logs: `docker compose logs prod`
- Verify database file ownership: `ls -l data/*.db` (should be 1001:1001)
- Check `.env.prod` is readable and has all required vars

### Reverse proxy returns 502
- Verify app is running: `docker compose ps`
- Check health endpoint: `docker exec <container-id> curl http://localhost:3000/health`

### TLS certificate won't issue
- Verify DNS is propagated: `nslookup kartoteka.example.com`
- Check Caddy logs for Let's Encrypt errors
- Ensure ports 80 and 443 are accessible from the internet
