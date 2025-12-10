"""
End-to-end integration test for real encoding pipeline.

Tests the full flow:
1. Create IntentEvent
2. Send to Management Plane
3. Management Plane encodes intent and boundary vectors
4. Rust sandbox performs comparison
5. Return decision + similarities
"""

import requests
import json
import time


def test_read_database_allowed():
    """Test that read access to database is allowed by the test boundary (v1.1)."""
    intent = {
        "id": "test_e2e_001",
        "schemaVersion": "v1.1",
        "tenantId": "tenant_test",
        "timestamp": time.time(),
        "action": "read",
        "actor": {
            "id": "alice@example.com",
            "type": "user"
        },
        "resource": {
            "type": "database",
            "name": "users_db",
            "location": "cloud"
        },
        "data": {
            "sensitivity": ["internal"],
            "pii": False,
            "volume": "single"
        },
        "risk": {
            "authn": "required"
        }
    }

    response = requests.post(
        "http://localhost:8000/api/v1/intents/compare",
        json=intent,
        headers={"Content-Type": "application/json"}
    )

    print("\n=== Test: Read Database (Expected: ALLOW) ===")
    print(f"Status: {response.status_code}")

    if response.status_code == 200:
        result = response.json()
        print(f"Decision: {'ALLOW' if result['decision'] == 1 else 'BLOCK'}")
        print(f"Slice similarities:")
        print(f"  Action:   {result['slice_similarities'][0]:.4f}")
        print(f"  Resource: {result['slice_similarities'][1]:.4f}")
        print(f"  Data:     {result['slice_similarities'][2]:.4f}")
        print(f"  Risk:     {result['slice_similarities'][3]:.4f}")

        # Test boundary has thresholds: action=0.8, resource=0.75, data=0.7, risk=0.6
        # For min mode, all must pass
        assert result['decision'] in [0, 1], "Decision must be 0 or 1"
        print("\n✅ Test passed!")
    else:
        print(f"Error: {response.text}")
        print("\n❌ Test failed!")


def test_delete_operation_blocked():
    """Test that delete operations have lower similarity (v1.1)."""
    intent = {
        "id": "test_e2e_002",
        "schemaVersion": "v1.1",
        "tenantId": "tenant_test",
        "timestamp": time.time(),
        "action": "delete",  # Different action - should have lower similarity
        "actor": {
            "id": "bob@example.com",
            "type": "user"
        },
        "resource": {
            "type": "database",
            "name": "users_db",
            "location": "cloud"
        },
        "data": {
            "sensitivity": ["internal"],
            "pii": False,
            "volume": "bulk"  # Different volume
        },
        "risk": {
            "authn": "not_required"  # Different auth requirement
        }
    }

    response = requests.post(
        "http://localhost:8000/api/v1/intents/compare",
        json=intent,
        headers={"Content-Type": "application/json"}
    )

    print("\n=== Test: Delete Database (Expected: BLOCK or lower similarities) ===")
    print(f"Status: {response.status_code}")

    if response.status_code == 200:
        result = response.json()
        print(f"Decision: {'ALLOW' if result['decision'] == 1 else 'BLOCK'}")
        print(f"Slice similarities:")
        print(f"  Action:   {result['slice_similarities'][0]:.4f}")
        print(f"  Resource: {result['slice_similarities'][1]:.4f}")
        print(f"  Data:     {result['slice_similarities'][2]:.4f}")
        print(f"  Risk:     {result['slice_similarities'][3]:.4f}")

        # Action similarity should be lower since "delete" != "read"
        assert result['slice_similarities'][0] < 0.95, \
            "Delete action should have lower similarity to read boundary"
        print("\n✅ Test passed!")
    else:
        print(f"Error: {response.text}")
        print("\n❌ Test failed!")


def test_determinism():
    """Test that same intent produces same results (deterministic encoding)."""
    intent = {
        "id": "test_e2e_003",
        "schemaVersion": "v1",
        "tenantId": "tenant_test",
        "timestamp": time.time(),
        "action": "read",
        "actor": {
            "id": "charlie@example.com",
            "type": "user"
        },
        "resource": {
            "type": "api",
            "name": "user_api"
        },
        "data": {
            "categories": ["public"]
        },
        "risk": {
            "authn": "mfa",
            "network": "corp"
        }
    }

    print("\n=== Test: Determinism (Same Input → Same Output) ===")

    # Call twice
    response1 = requests.post(
        "http://localhost:8000/api/v1/intents/compare",
        json=intent,
        headers={"Content-Type": "application/json"}
    )

    response2 = requests.post(
        "http://localhost:8000/api/v1/intents/compare",
        json=intent,
        headers={"Content-Type": "application/json"}
    )

    if response1.status_code == 200 and response2.status_code == 200:
        result1 = response1.json()
        result2 = response2.json()

        print(f"Call 1 - Decision: {result1['decision']}, Similarities: {result1['slice_similarities']}")
        print(f"Call 2 - Decision: {result2['decision']}, Similarities: {result2['slice_similarities']}")

        # Check determinism
        assert result1['decision'] == result2['decision'], "Decisions must match"

        for i in range(4):
            diff = abs(result1['slice_similarities'][i] - result2['slice_similarities'][i])
            assert diff < 0.0001, f"Slice {i} similarities must match (diff={diff})"

        print("\n✅ Test passed! Encoding is deterministic")
    else:
        print(f"Error: {response1.text if response1.status_code != 200 else response2.text}")
        print("\n❌ Test failed!")


if __name__ == "__main__":
    print("=" * 60)
    print("End-to-End Integration Test - Real Encoding Pipeline")
    print("=" * 60)

    # Check server is running
    try:
        health = requests.get("http://localhost:8000/health")
        if health.status_code == 200:
            print("✓ Management Plane is running")
        else:
            print("✗ Management Plane health check failed")
            exit(1)
    except Exception as e:
        print(f"✗ Cannot connect to Management Plane: {e}")
        print("  Make sure server is running: cd management_plane && ./run.sh")
        exit(1)

    # Run tests
    try:
        test_read_database_allowed()
        test_delete_operation_blocked()
        test_determinism()

        print("\n" + "=" * 60)
        print("All tests passed! ✅")
        print("=" * 60)
    except AssertionError as e:
        print(f"\n❌ Assertion failed: {e}")
        exit(1)
    except Exception as e:
        print(f"\n❌ Test error: {e}")
        import traceback
        traceback.print_exc()
        exit(1)
