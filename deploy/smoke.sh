#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://localhost:3000}"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

pass=0
fail=0

assert() {
    local desc="$1"
    shift
    local tmp
    tmp=$(mktemp)
    if "$@" >"$tmp" 2>&1; then
        pass=$((pass + 1))
    else
        echo "FAIL: $desc"
        sed 's/^/  /' "$tmp" | tail -10
        fail=$((fail + 1))
    fi
    rm -f "$tmp"
}

echo "Smoke start: $BASE_URL"

# --- Build CLI binary once ---
echo "Building ht CLI..."
HT="$PROJECT_ROOT/target/debug/ht"
cargo build -p hypertide-cli --bin ht 2>/dev/null

# --- Health ---
assert "health/live returns OK" \
    bash -c '[ "$(curl -sf '"$BASE_URL"'/health/live)" = "OK" ]'

assert "health/ready succeeds" \
    curl -sf "$BASE_URL/health/ready"

# --- Auth ---
RESP_FILE=$(mktemp)
trap 'rm -f "$RESP_FILE"' EXIT

curl -sf -X POST "$BASE_URL/v2/auth/exchange-key" \
    -H "Content-Type: application/json" \
    -d '{"api_key":"dev-master-key"}' > "$RESP_FILE"

assert "auth/exchange-key returns success" \
    jq -e '.success == true' "$RESP_FILE"

assert "auth/exchange-key has access_token" \
    jq -e '.data.access_token | length > 0' "$RESP_FILE"

curl -sf "$BASE_URL/v2/auth/verify" \
    -H "X-API-Key: dev-master-key" > "$RESP_FILE"

assert "auth/verify returns success" \
    jq -e '.success == true' "$RESP_FILE"

# --- CLI flow ---
SMOKE_ID=$(date +%s)
REPO="smoke-repo-$SMOKE_ID"
WORKSPACE="$(mktemp -d)/smoke-$SMOKE_ID"
mkdir -p "$WORKSPACE"
ASSET_PATH="Content/Smoke/hello.txt"

echo "hello from smoke $SMOKE_ID" > "$WORKSPACE/hello.txt"

assert "ht login" \
    "$HT" login \
        --server "$BASE_URL" \
        --token "dev-master-key" \
        --api-key-direct \
        --repo "$REPO" \
        --branch main

assert "ht branch create" \
    "$HT" branch create \
        --repo "$REPO" \
        --name "smoke-bootstrap"

assert "ht add" \
    "$HT" add \
        --file "$WORKSPACE/hello.txt" \
        --asset-path "$ASSET_PATH"

assert "ht submit" \
    "$HT" submit \
        --message "smoke submit $SMOKE_ID"

assert "ht sync" \
    "$HT" sync

assert "ht checkout" \
    "$HT" checkout

assert "ht status" \
    "$HT" status

assert "ht diff" \
    "$HT" diff

assert "checkout materialized asset" \
    test -f "$WORKSPACE/Content/Smoke/hello.txt"

echo ""
echo "Results: $pass passed, $fail failed"

rm -rf "$WORKSPACE"

if [ "$fail" -gt 0 ]; then
    echo "Smoke FAILED."
    exit 1
fi

echo "Smoke passed."
