# Quick Start Guide

## What's Ready Now

✅ **Semantic Sandbox (Rust)** - Fully functional FFI library

## Testing the Rust Sandbox

```bash
# Build the library
cd semantic-sandbox
cargo build --release

# Run tests
cargo test

# Test FFI from Python
python3 test_ffi.py
```

## Project Structure

```
mgmt-plane/                          # Root
├── semantic-sandbox/                # ✅ DONE - Rust CDylib
│   ├── src/
│   │   ├── lib.rs                   # FFI interface
│   │   └── compare.rs               # Comparison logic
│   ├── Cargo.toml
│   └── test_ffi.py                  # Python FFI validator
│
├── management-plane/                # TODO - FastAPI service
│   └── app/
│
├── control-plane/                   # TODO - React UI
│   └── src/
│
├── tupl_sdk/                        # TODO - Client SDKs
│   ├── typescript/
│   └── python/
│
├── docs/
├── scripts/
├── plan.md                          # Full implementation plan
├── STATUS.md                        # Current progress
└── QUICKSTART.md                    # This file
```

## Next: Management Plane Setup

The next priority is setting up the Python FastAPI service that will:
1. Load the Rust library via FFI (use `semantic-sandbox/test_ffi.py` as reference)
2. Expose REST endpoints for the Control Plane and SDKs
3. Implement encoding pipeline (Week 2)

### Prerequisites for Management Plane

```bash
# Install Python dependencies
cd management-plane
python3 -m venv venv
source venv/bin/activate
pip install fastapi uvicorn pydantic sentence-transformers numpy ctypes
```

### File Structure to Create

```
management-plane/
├── app/
│   ├── __init__.py
│   ├── main.py                      # FastAPI app
│   ├── types.py                     # Pydantic models (from plan.md)
│   ├── api/
│   │   ├── intents.py
│   │   ├── boundaries.py
│   │   ├── compare.py
│   │   └── telemetry.py
│   ├── matching/
│   │   └── sandbox_bridge.py        # FFI wrapper (adapt test_ffi.py)
│   └── config.py
├── requirements.txt
└── README.md
```

## FFI Integration Reference

The `semantic-sandbox/test_ffi.py` script shows exactly how to:
- Load the Rust library with `ctypes.CDLL()`
- Define matching Python structures (`VectorEnvelope`, `ComparisonResult`)
- Call Rust functions from Python
- Handle results

Copy this pattern into `management-plane/app/matching/sandbox_bridge.py`.

## Development Workflow

1. **Week 1 Focus**: Get all components talking to each other
   - Rust ✅ DONE
   - Python FastAPI skeleton (next)
   - React skeleton
   - SDK skeletons
   - End-to-end smoke test

2. **Week 2 Focus**: Real encoding + comparison logic
3. **Week 3 Focus**: Storage + boundaries
4. **Week 4 Focus**: Hardening + testing

## Running Components (when ready)

```bash
# Terminal 1: Management Plane
cd management-plane
source venv/bin/activate
uvicorn app.main:app --reload --port 8000

# Terminal 2: Control Plane
cd control-plane
npm run dev

# Terminal 3: Test SDK
cd tupl_sdk/python
python test_sdk.py
```

## Useful Commands

```bash
# Rebuild Rust library after changes
cd semantic-sandbox
cargo build --release

# Validate FFI still works
python3 test_ffi.py

# Run all Rust tests
cargo test -- --nocapture
```

## Getting Help

- See `plan.md` for full implementation details
- See `STATUS.md` for current progress
- See component READMEs for specific documentation
