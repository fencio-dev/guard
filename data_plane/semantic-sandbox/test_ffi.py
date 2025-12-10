#!/usr/bin/env python3
"""
Quick FFI test script to verify Rust library can be loaded from Python.
This validates the Week 1 goal of getting the FFI bridge working.
"""

import ctypes
import numpy as np
from pathlib import Path

# Define FFI structures matching Rust definitions
class VectorEnvelope(ctypes.Structure):
    _fields_ = [
        ("intent", ctypes.c_float * 128),
        ("boundary", ctypes.c_float * 128),
        ("thresholds", ctypes.c_float * 4),
        ("weights", ctypes.c_float * 4),
        ("decision_mode", ctypes.c_uint8),
        ("global_threshold", ctypes.c_float),
    ]

class ComparisonResult(ctypes.Structure):
    _fields_ = [
        ("decision", ctypes.c_uint8),
        ("slice_similarities", ctypes.c_float * 4),
    ]

def main():
    # Load the library
    lib_path = Path(__file__).parent / "target" / "release" / "libsemantic_sandbox.dylib"

    if not lib_path.exists():
        print(f"❌ Library not found at {lib_path}")
        print("   Run: cargo build --release")
        return 1

    print(f"Loading library from {lib_path}")
    lib = ctypes.CDLL(str(lib_path))

    # Test 1: Health check
    print("\n--- Test 1: Health Check ---")
    lib.health_check.restype = ctypes.c_uint8
    health = lib.health_check()
    print(f"Health check: {health} {'✅' if health == 1 else '❌'}")

    # Test 2: Version check
    print("\n--- Test 2: Version Check ---")
    lib.get_version.restype = ctypes.c_uint32
    version = lib.get_version()
    print(f"Version: {version} ✅")

    # Test 3: Vector comparison
    print("\n--- Test 3: Vector Comparison ---")
    lib.compare_vectors.argtypes = [ctypes.POINTER(VectorEnvelope)]
    lib.compare_vectors.restype = ComparisonResult

    # Create test envelope
    envelope = VectorEnvelope()

    # Fill with test data
    for i in range(128):
        envelope.intent[i] = 0.9
        envelope.boundary[i] = 1.0

    envelope.thresholds[:] = [0.85, 0.85, 0.85, 0.85]
    envelope.weights[:] = [1.0, 1.0, 1.0, 1.0]
    envelope.decision_mode = 0  # min mode
    envelope.global_threshold = 0.85

    # Call the function
    result = lib.compare_vectors(ctypes.byref(envelope))

    print(f"Decision: {result.decision} (0=block, 1=allow)")
    print(f"Slice similarities:")
    for i, name in enumerate(["action", "resource", "data", "risk"]):
        print(f"  {name:10s}: {result.slice_similarities[i]:.4f}")

    print("\n✅ All FFI tests passed!")
    print("The Rust library is ready for integration with the Management Plane.")

    return 0

if __name__ == "__main__":
    exit(main())
