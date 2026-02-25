import requests
import sys
import time
import json

BASE_URL = "http://localhost:3000"
DEV_MASTER_KEY = "dev-master-key"

def print_pass(msg):
    print(f"✅ {msg}")

def print_fail(msg):
    print(f"❌ {msg}")
    sys.exit(1)

def test_health():
    try:
        r = requests.get(f"{BASE_URL}/health")
        if r.status_code == 200:
            print_pass("Health check passed")
        else:
            print_fail(f"Health check failed: {r.status_code} {r.text}")
    except Exception as e:
        print_fail(f"Could not connect to server: {e}")

def test_auth():
    print(f"\n--- Testing Auth ---")
    # Verify Master Key
    headers = {"X-API-Key": DEV_MASTER_KEY}
    try:
        r = requests.get(f"{BASE_URL}/api/auth/verify", headers=headers)
        if r.status_code != 200:
            print_fail(f"Master key verify failed: {r.status_code}")
        
        data = r.json()
        if data["data"]["valid"] == True:
            print_pass("Master key verification passed")
        else:
            print_fail(f"Master key verification failed: {data}")

        # Generate New Key
        payload = {
            "owner_id": "test_user_01",
            "permissions": ["lock", "upload", "download"],
            "expires_in_days": 7
        }
        r = requests.post(f"{BASE_URL}/api/auth/generate", json=payload, headers=headers)
        if r.status_code == 201:
            new_key_data = r.json()["data"]
            new_key = new_key_data["key"]
            print_pass(f"Generated new key: {new_key[:8]}...")
            return new_key
        else:
            print_fail(f"Failed to generate key: {r.text}")

    except Exception as e:
        print_fail(f"Auth test exception: {e}")

def test_storage(api_key):
    print(f"\n--- Testing Storage ---")
    headers = {"X-API-Key": api_key}
    content = b"Content for Hypertide Test"
    files = {'file': ('test_file.txt', content)}
    
    try:
        # Upload
        r = requests.post(f"{BASE_URL}/api/upload", files=files, headers=headers)
        if r.status_code == 200:
            data = r.json()["data"]
            file_hash = data["hash"]
            print_pass(f"File uploaded, hash: {file_hash}")
        else:
            print_fail(f"Upload failed: {r.status_code} {r.text}")
            return None

        # Check Exists
        r = requests.get(f"{BASE_URL}/api/exists/{file_hash}", headers=headers)
        if r.json()["data"] == True:
            print_pass("File existence check passed")
        else:
            print_fail("File existence check failed")

        # Download
        r = requests.get(f"{BASE_URL}/api/download/{file_hash}", headers=headers)
        if r.status_code == 200 and r.content == content:
            print_pass("File download passed")
        else:
            print_fail("File download failed or content mismatch")
        
        return file_hash

    except Exception as e:
        print_fail(f"Storage test exception: {e}")

def test_lock(api_key):
    print(f"\n--- Testing Lock ---")
    headers = {"X-API-Key": api_key}
    file_path = "assets/hero.fbx"
    owner_id = "test_user_01"

    try:
        # Lock
        payload = {"file_path": file_path, "owner_id": owner_id}
        r = requests.post(f"{BASE_URL}/api/lock", json=payload, headers=headers)
        if r.status_code == 200:
            print_pass(f"Locked file: {file_path}")
        else:
            print_fail(f"Lock failed: {r.text}")

        # List Locks
        r = requests.get(f"{BASE_URL}/api/locks", headers=headers)
        locks = r.json()["data"]
        found = any(l["file_path"] == file_path for l in locks)
        if found:
            print_pass("List locks passed")
        else:
            print_fail("List locks failed - lock not found")

        # Unlock
        unlock_payload = {"file_path": file_path, "owner_id": owner_id}
        # Note: The API in lock.rs uses DELETE method
        # But wait, does requests.delete support json body? Yes.
        r = requests.delete(f"{BASE_URL}/api/unlock", json=unlock_payload, headers=headers)
        if r.status_code == 200:
            print_pass("Unlock passed")
        else:
            print_fail(f"Unlock failed: {r.text}")

    except Exception as e:
        print_fail(f"Lock test exception: {e}")

if __name__ == "__main__":
    print("🚀 Starting Hypertide Integration Tests...")
    test_health()
    new_key = test_auth()
    if new_key:
        test_storage(new_key)
        test_lock(new_key)
    print("\n✨ All tests passed successfully!")
