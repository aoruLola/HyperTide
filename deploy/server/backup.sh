#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-$SCRIPT_DIR/docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-$SCRIPT_DIR/.env.production}"
BACKUP_ROOT="${BACKUP_ROOT:-$SCRIPT_DIR/backups}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
BACKUP_DIR="$BACKUP_ROOT/$STAMP"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing $ENV_FILE. Create it from .env.production.example first." >&2
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

mkdir -p "$BACKUP_DIR"

docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" exec -T postgres \
  pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB" > "$BACKUP_DIR/postgres.sql"

if [[ -d "$SCRIPT_DIR/data/storage" ]]; then
  tar -czf "$BACKUP_DIR/storage.tar.gz" -C "$SCRIPT_DIR/data/storage" .
else
  mkdir -p "$SCRIPT_DIR/data/storage"
  tar -czf "$BACKUP_DIR/storage.tar.gz" -C "$SCRIPT_DIR/data/storage" .
fi

if [[ -d "$SCRIPT_DIR/keys" ]]; then
  tar -czf "$BACKUP_DIR/keys.tar.gz" -C "$SCRIPT_DIR/keys" .
fi

cat > "$BACKUP_DIR/manifest.json" <<JSON
{
  "created_at": "$STAMP",
  "compose_file": "$(basename "$COMPOSE_FILE")",
  "postgres_db": "$POSTGRES_DB",
  "storage_path": "deploy/server/data/storage",
  "includes": ["postgres.sql", "storage.tar.gz", "keys.tar.gz"]
}
JSON

(
  cd "$BACKUP_DIR"
  sha256sum postgres.sql storage.tar.gz manifest.json > SHA256SUMS
  if [[ -f keys.tar.gz ]]; then
    sha256sum keys.tar.gz >> SHA256SUMS
  fi
)

echo "Backup written to $BACKUP_DIR"
