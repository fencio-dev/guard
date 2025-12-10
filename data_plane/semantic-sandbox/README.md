# Semantic Sandbox

Rust CDylib for high-performance vector comparison in semantic security system.

## Overview

The Semantic Sandbox is a C-compatible dynamic library that performs vector similarity comparisons with configurable thresholds and aggregation modes. It receives 128-dimensional vectors (intent and boundary) and returns a binary decision (allow/block) along with per-slice similarity scores.

## Architecture

- **Input**: `VectorEnvelope` struct containing:
  - Intent vector (128 floats)
  - Boundary vector (128 floats)
  - Per-slice thresholds (4 floats)
  - Per-slice weights (4 floats)
  - Decision mode (0=min, 1=weighted-avg)
  - Global threshold (for weighted-avg mode)

- **Output**: `ComparisonResult` struct containing:
  - Decision (0=block, 1=allow)
  - Slice similarities (4 floats)

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Output

The compiled library will be at:
- **macOS**: `target/release/libsemantic_sandbox.dylib`
- **Linux**: `target/release/libsemantic_sandbox.so`
- **Windows**: `target/release/semantic_sandbox.dll`

## FFI Usage from Python

```python
import ctypes
import numpy as np
from pathlib import Path

# Load library
lib = ctypes.CDLL("target/release/libsemantic_sandbox.dylib")

# Define structures
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

# Configure function signature
lib.compare_vectors.argtypes = [ctypes.POINTER(VectorEnvelope)]
lib.compare_vectors.restype = ComparisonResult

# Use it
envelope = VectorEnvelope()
# ... populate fields ...
result = lib.compare_vectors(ctypes.byref(envelope))
print(f"Decision: {result.decision}")
print(f"Similarities: {list(result.slice_similarities)}")
```

## Development Roadmap

### Week 1 (Current)
- ✅ Basic FFI interface with dummy implementation
- ✅ Health check and version functions
- ✅ Test harness

### Week 2 (Next)
- [ ] Real slice-based comparison logic
- [ ] Min mode implementation (mandatory boundaries)
- [ ] Weighted-avg mode implementation (optional boundaries)
- [ ] Performance optimization

### Week 4
- [ ] Comprehensive error handling
- [ ] Edge case testing
- [ ] Performance benchmarks (target: <1ms per comparison)

## Testing

Run the test suite:

```bash
cargo test
```

Expected output for Week 1:
- All dummy tests pass
- FFI structures are correctly sized
- Health check returns 1

## Performance Targets

- Single comparison: < 1ms
- Batch of 100 comparisons: < 100ms
- Zero allocations after initialization

## Safety Notes

This library uses `unsafe` for FFI boundaries. Safety invariants:

1. Caller must provide valid pointer to `VectorEnvelope`
2. `VectorEnvelope` must be properly initialized
3. Library is reentrant but not thread-safe (caller must synchronize)
4. No memory is allocated or freed across FFI boundary
