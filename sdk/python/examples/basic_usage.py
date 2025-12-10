#!/usr/bin/env python3
"""
Basic usage example for Tupl SDK.

Demonstrates:
- Creating an IntentEvent
- Sending to Management Plane
- Interpreting the response
"""

import time
import uuid
from tupl import TuplClient, IntentEvent, Actor, Resource, Data, Risk


def main():
    """
    Basic example: capture a single intent and check the result.
    """
    print("=" * 60)
    print("Tupl SDK - Basic Usage Example")
    print("=" * 60)

    # 1. Create client
    print("\n[1] Initializing TuplClient...")
    client = TuplClient(
        endpoint="http://localhost:8000",
        timeout=10.0
    )
    print("✓ Client initialized")

    # 2. Create an IntentEvent
    print("\n[2] Creating IntentEvent...")
    event = IntentEvent(
        id=f"evt-{uuid.uuid4()}",
        schemaVersion="v1",
        tenantId="tenant-123",
        timestamp=time.time(),
        actor=Actor(id="user-alice", type="user"),
        action="read",
        resource=Resource(
            type="database",
            name="users_db",
            location="cloud"
        ),
        data=Data(
            categories=["pii"],
            pii=True,
            volume="row"
        ),
        risk=Risk(
            authn="mfa",
            network="corp",
            timeOfDay=14
        )
    )
    print(f"✓ Created IntentEvent: {event.id}")
    print(f"  - Action: {event.action}")
    print(f"  - Resource: {event.resource.type}/{event.resource.name}")
    print(f"  - Data categories: {event.data.categories}")
    print(f"  - Authentication: {event.risk.authn}")

    # 3. Send to Management Plane
    print("\n[3] Sending IntentEvent to Management Plane...")
    result = client.capture(event)

    # 4. Interpret response
    if result:
        print("✓ Received response from Management Plane:")
        print(f"  - Decision: {'ALLOW' if result.decision == 1 else 'BLOCK'}")
        print(f"  - Slice similarities:")
        print(f"    - Action:   {result.slice_similarities[0]:.3f}")
        print(f"    - Resource: {result.slice_similarities[1]:.3f}")
        print(f"    - Data:     {result.slice_similarities[2]:.3f}")
        print(f"    - Risk:     {result.slice_similarities[3]:.3f}")

        if result.decision == 1:
            print("\n✅ Intent ALLOWED - operation may proceed")
        else:
            print("\n🚫 Intent BLOCKED - operation denied")
    else:
        print("✗ Failed to get response from Management Plane")
        print("  Check that the Management Plane is running on http://localhost:8000")

    # 5. Clean up
    print("\n[4] Closing client...")
    client.close()
    print("✓ Client closed")

    print("\n" + "=" * 60)
    print("Example complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
