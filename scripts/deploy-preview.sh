#!/usr/bin/env bash
set -euo pipefail

IMAGE="ghcr.io/jpalczewski/kartoteka-a1:preview"

echo "→ Logging into GHCR..."
gh auth token | docker login ghcr.io -u "$(gh api user --jq '.login')" --password-stdin

echo "→ Building AMD64 image..."
docker buildx build \
  --platform linux/amd64 \
  --build-arg CARGO_PROFILE=debug \
  --tag "$IMAGE" \
  --push \
  .

echo "→ Triggering Coolify deploy..."
curl -fsSL -X GET "$COOLIFY_WEBHOOK_PREVIEW" \
  -H "Authorization: Bearer $COOLIFY_TOKEN"

echo "✓ Done: $IMAGE deployed."
