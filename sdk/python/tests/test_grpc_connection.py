#!/usr/bin/env python3
"""
Quick test to verify gRPC connection to Data Plane works.
"""

import sys
import time
sys.path.insert(0, 'tupl_sdk/python')

from tupl import DataPlaneClient, IntentEvent, Actor, Resource, Data, Risk

def test_grpc_connection():
    """Test basic gRPC connection and enforce call."""

    print("=" * 80)
    print("Testing gRPC Connection to Data Plane")
    print("=" * 80)

    # Create Data Plane client
    client = DataPlaneClient(url="localhost:50051", timeout=10.0)
    print(f"✓ DataPlaneClient created: {client.url}")

    # Create a simple IntentEvent (v1.3)
    event = IntentEvent(
        id="test-001",
        schemaVersion="v1.3",
        tenantId="test-tenant",
        timestamp=time.time(),
        actor=Actor(id="test-agent", type="agent"),
        action="read",
        resource=Resource(type="database", name="test_tool", location="cloud"),
        data=Data(sensitivity=["internal"], pii=False, volume="single"),
        risk=Risk(authn="required"),
        # v1.3 fields
        layer="L4",
        tool_name="test_tool",
        tool_method="query",
        tool_params={"query": "test"}
    )
    print(f"✓ IntentEvent created: layer={event.layer}, tool={event.tool_name}")

    # Call enforce
    print("\nCalling DataPlane.Enforce via gRPC...")
    try:
        result = client.enforce(event)
        print(f"✓ Enforce call succeeded!")
        print(f"  - Decision: {result.decision} ({'ALLOW' if result.decision == 1 else 'BLOCK'})")
        print(f"  - Similarities: {result.slice_similarities}")
        print(f"  - Rules evaluated: {result.boundaries_evaluated}")
        print(f"  - Evidence count: {len(result.evidence)}")

        if result.evidence:
            print("\n  Evidence:")
            for ev in result.evidence:
                print(f"    - {ev.boundary_id}: {ev.boundary_name} → {'ALLOW' if ev.decision == 1 else 'BLOCK'}")

        return True

    except Exception as e:
        print(f"✗ Enforce call failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_grpc_connection()
    sys.exit(0 if success else 1)
