#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-$SCRIPT_DIR/docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-$SCRIPT_DIR/.env.production}"
BACKUP_DIR="${1:-}"

if [[ -z "$BACKUP_DIR" ]]; then
  echo "Usage: restore.sh <backup-directory>" >&2
  exit 1
fi
if [[ ! -d "$BACKUP_DIR" ]]; then
  echo "Backup directory does not exist: $BACKUP_DIR" >&2
  exit 1
fi
if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing $ENV_FILE. Create it from .env.production.example first." >&2
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

STORAGE_DIR="$SCRIPT_DIR/data/storage"
mkdir -p "$STORAGE_DIR"
if find "$STORAGE_DIR" -mindepth 1 -print -quit | grep -q .; then
  echo "Refusing to restore storage into a non-empty directory: $STORAGE_DIR" >&2
  echo "Start from a fresh target or move existing data aside manually after review." >&2
  exit 1
fi

TABLE_COUNT="$(
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" exec -T postgres \
    psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc \
    "select count(*) from information_schema.tables where table_schema = 'public';"
)"
if [[ "${TABLE_COUNT//[[:space:]]/}" != "0" && "${RESTORE_ALLOW_NON_EMPTY_DB:-false}" != "true" ]]; then
  echo "Refusing to restore into a database that already has public tables." >&2
  echo "Set RESTORE_ALLOW_NON_EMPTY_DB=true only after you have reviewed the target." >&2
  exit 1
fi

tar -xzf "$BACKUP_DIR/storage.tar.gz" -C "$STORAGE_DIR"
if [[ -f "$BACKUP_DIR/keys.tar.gz" ]]; then
  mkdir -p "$SCRIPT_DIR/keys"
  tar -xzf "$BACKUP_DIR/keys.tar.gz" -C "$SCRIPT_DIR/keys"
fi

docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" exec -T postgres \
  psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" < "$BACKUP_DIR/postgres.sql"

echo "Restore completed from $BACKUP_DIR"
