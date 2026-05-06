#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SOURCE_DIR/../.." && pwd)"
BACKUP_DIR="${1:-}"
if [[ -z "$BACKUP_DIR" ]]; then
  BACKUP_DIR="$(find "$SOURCE_DIR/backups" -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)"
fi
if [[ -z "$BACKUP_DIR" || ! -d "$BACKUP_DIR" ]]; then
  echo "No backup directory found for restore smoke." >&2
  exit 1
fi

RESTORE_DIR="${RESTORE_DIR:-$SOURCE_DIR/ci-restore-target}"
mkdir -p "$RESTORE_DIR"
cp "$SOURCE_DIR/docker-compose.prod.yml" "$RESTORE_DIR/"
cp "$SOURCE_DIR/restore.sh" "$RESTORE_DIR/"
cp "$SOURCE_DIR/smoke.sh" "$RESTORE_DIR/"
cp "$SOURCE_DIR/.env.production" "$RESTORE_DIR/"
mkdir -p "$RESTORE_DIR/data/postgres" "$RESTORE_DIR/data/storage" "$RESTORE_DIR/keys" "$RESTORE_DIR/backups"
chmod -R 777 "$RESTORE_DIR/data" "$RESTORE_DIR/backups"

cat > "$RESTORE_DIR/docker-compose.ci.yml" <<YAML
services:
  hypertide:
    image: hypertide-server:ci
    build:
      context: $REPO_ROOT
      dockerfile: deploy/server/Dockerfile.prod
    ports:
      - "3001:3000"

  caddy:
    profiles:
      - manual
YAML

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-hypertide_restore_ci}"
export COMPOSE_PROJECT_NAME

docker compose \
  -f "$RESTORE_DIR/docker-compose.prod.yml" \
  -f "$RESTORE_DIR/docker-compose.ci.yml" \
  --env-file "$RESTORE_DIR/.env.production" \
  up -d postgres

for _ in $(seq 1 30); do
  if docker compose \
    -f "$RESTORE_DIR/docker-compose.prod.yml" \
    -f "$RESTORE_DIR/docker-compose.ci.yml" \
    --env-file "$RESTORE_DIR/.env.production" \
    exec -T postgres pg_isready -U hypertide -d hypertide >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

bash "$RESTORE_DIR/restore.sh" "$BACKUP_DIR"
chmod -R 777 "$RESTORE_DIR/data/storage"
chmod -R a+rX "$RESTORE_DIR/keys"

docker compose \
  -f "$RESTORE_DIR/docker-compose.prod.yml" \
  -f "$RESTORE_DIR/docker-compose.ci.yml" \
  --env-file "$RESTORE_DIR/.env.production" \
  up -d --build hypertide

BASE_URL=http://127.0.0.1:3001 bash "$RESTORE_DIR/smoke.sh"
