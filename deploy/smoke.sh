#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${1:-http://localhost:3000}"

pass=0
fail=0

assert() {
    local desc="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        pass=$((pass + 1))
    else
        echo "FAIL: $desc"
        fail=$((fail + 1))
    fi
}

echo "Smoke start: $BASE_URL"

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

cd "$WORKSPACE"

assert "ht login" \
    cargo run -p hypertide-cli --bin ht -- login \
        --server "$BASE_URL" \
        --token "dev-master-key" \
        --api-key-direct \
        --repo "$REPO" \
        --branch main

assert "ht branch create" \
    cargo run -p hypertide-cli --bin ht -- branch create \
        --repo "$REPO" \
        --name "smoke-bootstrap"

assert "ht add" \
    cargo run -p hypertide-cli --bin ht -- add \
        --file "$WORKSPACE/hello.txt" \
        --asset-path "$ASSET_PATH"

assert "ht submit" \
    cargo run -p hypertide-cli --bin ht -- submit \
        --message "smoke submit $SMOKE_ID"

assert "ht sync" \
    cargo run -p hypertide-cli --bin ht -- sync

assert "ht checkout" \
    cargo run -p hypertide-cli --bin ht -- checkout

assert "ht status" \
    cargo run -p hypertide-cli --bin ht -- status

assert "ht diff" \
    cargo run -p hypertide-cli --bin ht -- diff

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
