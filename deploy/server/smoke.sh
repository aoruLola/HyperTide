#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:3000}"
ATTEMPTS="${ATTEMPTS:-30}"
SLEEP_SECONDS="${SLEEP_SECONDS:-2}"

wait_for_url() {
  local path="$1"
  local attempt
  for attempt in $(seq 1 "$ATTEMPTS"); do
    if curl -fsS "$BASE_URL$path" >/dev/null; then
      return 0
    fi
    sleep "$SLEEP_SECONDS"
  done
  echo "Timed out waiting for $BASE_URL$path" >&2
  return 1
}

wait_for_url "/health/live"
wait_for_url "/health/ready"
curl -fsS "$BASE_URL/metrics" | grep -q "hypertide_http_requests_total"

echo "HyperTide smoke checks passed for $BASE_URL"
