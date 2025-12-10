# Development Setup

## Prerequisites

- **uv** (Python package manager): [Install instructions](https://docs.astral.sh/uv/)
- **Rust** (for semantic-sandbox): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

## Python Environment Setup

This project uses **Python 3.14** managed with **uv** and **pyproject.toml**.

### Quick Start

```bash
# Install all dependencies for the workspace
uv sync --all-packages

# Verify Python version
uv run python --version  # Should show Python 3.14.0
```

### Project Structure

This is a monorepo with the following Python packages:

- **management-plane/** - FastAPI application for policy enforcement
- **tupl_sdk/python/** - Python SDK for capturing IntentEvents

### Running Tests

```bash
# Run all tests in the workspace
uv run pytest

# Run tests for a specific package
cd management-plane
uv run pytest tests/test_types.py -v

# Run tests with coverage
uv run pytest --cov=app --cov-report=html
```

### Adding Dependencies

```bash
# Add a dependency to management-plane
cd management-plane
uv add <package-name>

# Add a dev dependency
uv add --dev <package-name>

# Add to Python SDK
cd tupl_sdk/python
uv add <package-name>
```

### Virtual Environment

uv automatically creates and manages a virtual environment in `.venv/` at the project root.

```bash
# Activate the virtual environment (optional, uv run handles this)
source .venv/bin/activate

# Deactivate
deactivate
```

### IDE Setup

**VS Code:**
1. Install the Python extension
2. Select the interpreter: `.venv/bin/python`
3. The IDE will automatically use Python 3.14

**PyCharm:**
1. File → Project Structure → SDKs
2. Add SDK → Python SDK → Existing environment
3. Select `.venv/bin/python`

### Building the Rust Component

```bash
cd semantic-sandbox
cargo build --release

# Test FFI interface
python test_ffi.py
```

## Common Commands

```bash
# Sync all packages (install/update dependencies)
uv sync --all-packages

# Run a Python script
uv run python script.py

# Run pytest
uv run pytest

# Run the management plane server (when implemented)
cd management-plane
uv run uvicorn app.main:app --reload

# Format code (when ruff is added)
uv run ruff format .

# Lint code (when ruff is added)
uv run ruff check .
```

## Troubleshooting

### "Package not found" errors
```bash
# Re-sync all packages
uv sync --all-packages
```

### Python version mismatch
```bash
# Check the .python-version file exists
cat .python-version  # Should show 3.14

# uv will automatically download Python 3.14 if needed
```

### Tests fail to import modules
```bash
# Make sure you're running tests from the package directory
cd management-plane
uv run pytest tests/
```

## Dependency Files

- **pyproject.toml** (root) - Workspace configuration
- **management-plane/pyproject.toml** - Management plane dependencies
- **tupl_sdk/python/pyproject.toml** - SDK dependencies
- **.python-version** - Python version lock (3.14)
- **uv.lock** - Dependency lock file (auto-generated, not committed)

## Migration from requirements.txt

We've migrated from `requirements.txt` to `pyproject.toml` for better dependency management and Python 3.14 support. All dependencies are now:

- **Explicitly versioned** in pyproject.toml
- **Locked** by uv for reproducible builds
- **Organized** by package (management-plane, SDK)
- **Separated** into production and dev dependencies

No manual `pip install` needed - `uv sync` handles everything!
