import json
import sys
import time
from typing import Any, Dict, Optional, Tuple

import requests

BASE_URL = "http://localhost:3000"
DEV_MASTER_KEY = "dev-master-key"


def print_pass(msg: str) -> None:
    print(f"[PASS] {msg}")


def print_fail(msg: str) -> None:
    print(f"[FAIL] {msg}")
    sys.exit(1)


def http_json(
    method: str,
    path: str,
    *,
    headers: Optional[Dict[str, str]] = None,
    payload: Optional[Dict[str, Any]] = None,
    files: Optional[Dict[str, Tuple[str, bytes]]] = None,
    expected_status: Optional[int] = None,
) -> Tuple[requests.Response, Dict[str, Any]]:
    url = f"{BASE_URL}{path}"
    kwargs: Dict[str, Any] = {"headers": headers or {}, "timeout": 15}
    if payload is not None:
        kwargs["json"] = payload
    if files is not None:
        kwargs["files"] = files

    resp = requests.request(method=method, url=url, **kwargs)
    if expected_status is not None and resp.status_code != expected_status:
        print_fail(
            f"{method} {path} expected {expected_status}, got {resp.status_code}: {resp.text}"
        )

    try:
        data = resp.json()
    except Exception as exc:
        print_fail(f"{method} {path} returned non-JSON: {exc}; body={resp.text}")
    return resp, data


def assert_success_envelope(data: Dict[str, Any], hint: str) -> Dict[str, Any]:
    if data.get("success") is not True:
        print_fail(f"{hint} failed: {json.dumps(data, ensure_ascii=False)}")
    return data.get("data")


def test_health() -> None:
    print("\n--- Testing Health ---")
    health = requests.get(f"{BASE_URL}/health", timeout=10)
    if health.status_code != 200:
        print_fail(f"health failed: {health.status_code} {health.text}")
    print_pass("health passed")

    live = requests.get(f"{BASE_URL}/health/live", timeout=10)
    if live.status_code != 200 or live.text.strip() != "OK":
        print_fail(f"health/live failed: {live.status_code} {live.text}")
    print_pass("health/live passed")

    ready = requests.get(f"{BASE_URL}/health/ready", timeout=10)
    if ready.status_code != 200:
        print_fail(f"health/ready failed: {ready.status_code} {ready.text}")
    print_pass("health/ready passed")


def test_auth() -> Dict[str, str]:
    print("\n--- Testing Auth (API Key + JWT) ---")

    master_headers = {"X-API-Key": DEV_MASTER_KEY}
    _, verify_data = http_json("GET", "/v2/auth/verify", headers=master_headers, expected_status=200)
    verify_payload = assert_success_envelope(verify_data, "master verify")
    if verify_payload.get("valid") is not True:
        print_fail(f"master verify invalid: {verify_data}")
    print_pass("master key verify passed")

    _, exchange_data = http_json(
        "POST",
        "/v2/auth/exchange-key",
        payload={"api_key": DEV_MASTER_KEY},
        expected_status=200,
    )
    pair1 = assert_success_envelope(exchange_data, "exchange-key")
    refresh_1 = pair1["refresh_token"]
    print_pass("exchange-key passed")

    _, refresh_data = http_json(
        "POST",
        "/v2/auth/refresh",
        payload={"refresh_token": refresh_1},
        expected_status=200,
    )
    pair2 = assert_success_envelope(refresh_data, "refresh")
    refresh_2 = pair2["refresh_token"]
    print_pass("refresh rotation passed")

    _, replay_old = http_json(
        "POST",
        "/v2/auth/refresh",
        payload={"refresh_token": refresh_1},
        expected_status=401,
    )
    if replay_old.get("success") is not False:
        print_fail("refresh replay should fail")
    print_pass("refresh replay protection passed")

    _, replay_family = http_json(
        "POST",
        "/v2/auth/refresh",
        payload={"refresh_token": refresh_2},
        expected_status=401,
    )
    if replay_family.get("success") is not False:
        print_fail("refresh family revoke should fail")
    print_pass("refresh family revoke passed")

    _, fresh_exchange = http_json(
        "POST",
        "/v2/auth/exchange-key",
        payload={"api_key": DEV_MASTER_KEY},
        expected_status=200,
    )
    pair3 = assert_success_envelope(fresh_exchange, "exchange-key after replay")
    _, revoke_data = http_json(
        "POST",
        "/v2/auth/revoke-refresh",
        payload={"refresh_token": pair3["refresh_token"]},
        expected_status=200,
    )
    revoke_payload = assert_success_envelope(revoke_data, "revoke-refresh")
    if revoke_payload.get("revoked") is not True:
        print_fail("revoke-refresh did not revoke token")
    print_pass("revoke-refresh passed")

    bearer_headers = {"Authorization": f"Bearer {pair3['access_token']}"}
    _, locks_data = http_json("GET", "/v2/locks", headers=bearer_headers, expected_status=200)
    assert_success_envelope(locks_data, "bearer access")
    print_pass("bearer authenticated request passed")

    owner = f"test_user_{int(time.time())}"
    _, generate_data = http_json(
        "POST",
        "/v2/auth/generate",
        headers=master_headers,
        payload={
            "owner_id": owner,
            "permissions": ["lock", "upload", "download"],
            "expires_in_days": 7,
        },
        expected_status=201,
    )
    generated = assert_success_envelope(generate_data, "generate key")
    print_pass("generate key passed")
    return {"api_key": generated["key"], "owner": owner}


def test_storage(api_key: str) -> Dict[str, str]:
    print("\n--- Testing Storage ---")
    headers = {"X-API-Key": api_key}

    files_1 = {"file": ("asset-a.bin", b"Asset-A-Content")}
    _, upload_1 = http_json(
        "POST", "/v2/storage/upload", headers=headers, files=files_1, expected_status=200
    )
    hash_1 = assert_success_envelope(upload_1, "upload-1")["hash"]
    print_pass(f"upload-1 passed: {hash_1}")

    files_2 = {"file": ("asset-b.bin", b"Asset-B-Content")}
    _, upload_2 = http_json(
        "POST", "/v2/storage/upload", headers=headers, files=files_2, expected_status=200
    )
    hash_2 = assert_success_envelope(upload_2, "upload-2")["hash"]
    print_pass(f"upload-2 passed: {hash_2}")

    _, exists_data = http_json(
        "GET", f"/v2/storage/exists/{hash_1}", headers=headers, expected_status=200
    )
    exists_value = assert_success_envelope(exists_data, "exists")
    if exists_value is not True:
        print_fail("exists check failed")
    print_pass("exists passed")

    download = requests.get(f"{BASE_URL}/v2/storage/download/{hash_1}", headers=headers, timeout=15)
    if download.status_code != 200 or download.content != b"Asset-A-Content":
        print_fail("download content mismatch")
    print_pass("download passed")
    return {"hash_1": hash_1, "hash_2": hash_2}


def test_lock(api_key: str, owner: str) -> None:
    print("\n--- Testing Lock ---")
    headers = {"X-API-Key": api_key}
    file_path = f"assets/lock-{int(time.time())}.uasset"

    _, lock_data = http_json(
        "POST",
        "/v2/locks/acquire",
        headers=headers,
        payload={"file_path": file_path, "owner_id": owner},
        expected_status=200,
    )
    assert_success_envelope(lock_data, "lock acquire")
    print_pass("lock acquire passed")

    _, list_data = http_json("GET", "/v2/locks", headers=headers, expected_status=200)
    locks = assert_success_envelope(list_data, "list locks")
    if not any(item.get("file_path") == file_path for item in locks):
        print_fail("lock entry missing in list")
    print_pass("list locks passed")

    _, unlock_data = http_json(
        "POST",
        "/v2/locks/release",
        headers=headers,
        payload={"file_path": file_path, "owner_id": owner},
        expected_status=200,
    )
    assert_success_envelope(unlock_data, "unlock")
    print_pass("unlock passed")


def test_versioning(api_key: str, owner: str, hash_1: str, hash_2: str) -> None:
    print("\n--- Testing Versioning ---")
    headers = {"X-API-Key": api_key}
    repo_id = f"repo_{int(time.time())}"
    branch = "main"
    asset_path = "assets/hero.fbx"

    _, submit_1 = http_json(
        "POST",
        "/v2/changesets",
        headers=headers,
        payload={
            "repo_id": repo_id,
            "branch": branch,
            "base_changeset_id": "ROOT",
            "kind": "normal",
            "author": owner,
            "message": "initial commit",
            "assets": [{"path": asset_path, "blob_hash": hash_1}],
        },
        expected_status=201,
    )
    c1 = assert_success_envelope(submit_1, "submit-1")["changeset_id"]
    print_pass(f"submit-1 passed: {c1}")

    _, submit_2 = http_json(
        "POST",
        "/v2/changesets",
        headers=headers,
        payload={
            "repo_id": repo_id,
            "branch": branch,
            "base_changeset_id": c1,
            "kind": "normal",
            "author": owner,
            "message": "second commit",
            "assets": [{"path": asset_path, "blob_hash": hash_2}],
        },
        expected_status=201,
    )
    c2 = assert_success_envelope(submit_2, "submit-2")["changeset_id"]
    print_pass(f"submit-2 passed: {c2}")

    _, stale_submit = http_json(
        "POST",
        "/v2/changesets",
        headers=headers,
        payload={
            "repo_id": repo_id,
            "branch": branch,
            "base_changeset_id": c1,
            "kind": "normal",
            "author": owner,
            "message": "stale should fail",
            "assets": [{"path": asset_path, "blob_hash": hash_1}],
        },
        expected_status=409,
    )
    if stale_submit.get("success") is not False:
        print_fail("stale base should return conflict")
    print_pass("CAS conflict check passed")

    _, history_data = http_json(
        "GET", f"/v2/history/{repo_id}?branch={branch}&limit=20", headers=headers, expected_status=200
    )
    history = assert_success_envelope(history_data, "history")
    if len(history.get("items", [])) < 2:
        print_fail("history should include >= 2 changesets")
    print_pass("history passed")

    _, rollback_data = http_json(
        "POST",
        "/v2/rollback",
        headers=headers,
        payload={
            "repo_id": repo_id,
            "branch": branch,
            "target_changeset_id": c1,
            "author": owner,
            "message": "rollback to c1",
        },
        expected_status=201,
    )
    assert_success_envelope(rollback_data, "rollback")
    print_pass("rollback passed")

    _, sync_data = http_json(
        "GET", f"/v2/sync/{repo_id}?branch={branch}", headers=headers, expected_status=200
    )
    sync_payload = assert_success_envelope(sync_data, "sync")
    assets = sync_payload.get("assets", [])
    hero = next((item for item in assets if item.get("path") == asset_path), None)
    if hero is None or hero.get("blob_hash") != hash_1:
        print_fail("sync result mismatch after rollback")
    print_pass("sync after rollback passed")


if __name__ == "__main__":
    print("Starting HyperTide M8 integration tests...")
    test_health()
    auth_ctx = test_auth()
    storage_ctx = test_storage(auth_ctx["api_key"])
    test_lock(auth_ctx["api_key"], auth_ctx["owner"])
    test_versioning(
        auth_ctx["api_key"], auth_ctx["owner"], storage_ctx["hash_1"], storage_ctx["hash_2"]
    )
    print("\nAll M8 integration tests passed.")
