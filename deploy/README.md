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

From the repository's `deploy/` directory, copy the templates to `/srv/kartoteka/`:

```bash
# As deploy user (or via ssh)
cd /srv/kartoteka
git clone https://github.com/jpalczewski/kartoteka-a1.git repo
cp repo/deploy/compose.yml .
cp repo/deploy/Caddyfile .
cp repo/deploy/.env.example .
```

Or manually copy `compose.yml`, `Caddyfile`, and `.env.example` from this repo.

### 4. Create per-environment env files

```bash
cd /srv/kartoteka

# Copy template for each environment
cp .env.example .env.prod
cp .env.example .env.dev
cp .env.example .env.preview
```

Edit each file with environment-specific values:

**`.env.prod`** (production):
```bash
KARTOTEKA_DOMAIN=kartoteka.example.com
DATABASE_URL=sqlite:///app/kartoteka.db?mode=rwc
OAUTH_SIGNING_SECRET=<generate-32-char-random-string>
PUBLIC_BASE_URL=https://kartoteka.example.com
APP_ENV=production
BIND_ADDR=0.0.0.0:3000
RUST_LOG=info
```

**`.env.dev`** (development):
```bash
KARTOTEKA_DOMAIN=kartoteka.example.com
DATABASE_URL=sqlite:///app/kartoteka.db?mode=rwc
OAUTH_SIGNING_SECRET=<different-32-char-random-string>
PUBLIC_BASE_URL=https://dev.kartoteka.example.com
APP_ENV=development
BIND_ADDR=0.0.0.0:3000
RUST_LOG=debug
```

**`.env.preview`** (preview/PR):
```bash
KARTOTEKA_DOMAIN=kartoteka.example.com
DATABASE_URL=sqlite:///app/kartoteka.db?mode=rwc
OAUTH_SIGNING_SECRET=<different-32-char-random-string>
PUBLIC_BASE_URL=https://pr.kartoteka.example.com
APP_ENV=staging
BIND_ADDR=0.0.0.0:3000
RUST_LOG=info
```

Set restrictive permissions:

```bash
chmod 600 .env.prod .env.dev .env.preview
```

### 5. Create root .env for Caddy

```bash
cd /srv/kartoteka
printf "KARTOTEKA_DOMAIN=kartoteka.example.com\n" > .env
```

This allows Caddy to read the domain from the compose environment.

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
