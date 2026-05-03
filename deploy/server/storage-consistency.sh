#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-$SCRIPT_DIR/docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-$SCRIPT_DIR/.env.production}"
STORAGE_OBJECTS="${STORAGE_OBJECTS:-$SCRIPT_DIR/data/storage/objects}"
OUTPUT_FORMAT="${OUTPUT_FORMAT:-human}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing $ENV_FILE. Create it from .env.production.example first." >&2
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

declare -A storage_hashes=()
declare -A db_hashes=()
missing=()
orphans=()

if [[ -d "$STORAGE_OBJECTS" ]]; then
  while IFS= read -r object_path; do
    relative="${object_path#"$STORAGE_OBJECTS"/}"
    hash="${relative//\//}"
    storage_hashes["$hash"]=1
  done < <(find "$STORAGE_OBJECTS" -mindepth 2 -type f)
fi

columns="$(
  docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" exec -T postgres \
    psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc \
    "select table_name || '.' || column_name from information_schema.columns where table_schema = 'public' and column_name in ('hash', 'blob_hash', 'content_hash', 'object_hash') order by table_name, column_name;"
)"

while IFS= read -r ref; do
  [[ -z "$ref" ]] && continue
  table="${ref%.*}"
  column="${ref#*.}"
  values="$(
    docker compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" exec -T postgres \
      psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc \
      "select distinct \"$column\" from public.\"$table\" where \"$column\" is not null and length(\"$column\") >= 3;"
  )"
  while IFS= read -r hash; do
    [[ -z "$hash" ]] && continue
    db_hashes["$hash"]=1
  done <<< "$values"
done <<< "$columns"

for hash in "${!db_hashes[@]}"; do
  if [[ -z "${storage_hashes[$hash]+x}" ]]; then
    missing+=("$hash")
  fi
done

for hash in "${!storage_hashes[@]}"; do
  if [[ -z "${db_hashes[$hash]+x}" ]]; then
    orphans+=("$hash")
  fi
done

if [[ "$OUTPUT_FORMAT" == "json" ]]; then
  printf '{"storage_objects":%s,"db_references":%s,"missing_objects":%s,"orphan_objects":%s}\n' \
    "${#storage_hashes[@]}" "${#db_hashes[@]}" "${#missing[@]}" "${#orphans[@]}"
else
  echo "Storage objects: ${#storage_hashes[@]}"
  echo "DB blob references: ${#db_hashes[@]}"
  echo "Missing objects referenced by DB: ${#missing[@]}"
  echo "Orphan storage objects: ${#orphans[@]}"
  if [[ "${#missing[@]}" -gt 0 ]]; then
    echo "Missing sample:"
    printf '  %s\n' "${missing[@]:0:20}"
  fi
  if [[ "${#orphans[@]}" -gt 0 ]]; then
    echo "Orphan sample:"
    printf '  %s\n' "${orphans[@]:0:20}"
  fi
fi
