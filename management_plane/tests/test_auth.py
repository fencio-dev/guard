import os
from unittest.mock import patch

import pytest
from fastapi.testclient import TestClient
from jose import jwt

from app.main import app

# --- Test Data ---
SECRET_KEY = "test-secret"
ALGORITHM = "HS256"
TEST_USER_ID = "test-user-123"

def create_test_token(payload: dict) -> str:
    """Creates a JWT for testing."""
    return jwt.encode(payload, SECRET_KEY, algorithm=ALGORITHM)

# --- Fixtures ---

@pytest.fixture(scope="module")
def client() -> TestClient:
    """Test client for the FastAPI application."""
    # Mock environment variables for auth
    os.environ["SUPABASE_URL"] = "https://test.supabase.co"
    os.environ["SUPABASE_JWT_SECRET"] = SECRET_KEY
    os.environ["SUPABASE_SERVICE_KEY"] = "service-role-key"

    from app import auth as auth_module

    auth_module.SUPABASE_URL = os.environ["SUPABASE_URL"]
    auth_module.SUPABASE_JWT_SECRET = os.environ["SUPABASE_JWT_SECRET"]
    auth_module.SUPABASE_SERVICE_KEY = os.environ["SUPABASE_SERVICE_KEY"]
    auth_module.get_supabase_service_client.cache_clear()
    
    with TestClient(app) as c:
        yield c
    
    del os.environ["SUPABASE_URL"]
    del os.environ["SUPABASE_JWT_SECRET"]
    del os.environ["SUPABASE_SERVICE_KEY"]

# --- Tests ---

def test_boundaries_requires_token(client: TestClient):
    """
    Protected endpoints should reject missing credentials.
    """
    response = client.get("/api/v1/boundaries")
    assert response.status_code == 401
    assert response.json()["detail"] in {"Not authenticated", "Could not validate credentials"}

def test_boundaries_invalid_token(client: TestClient):
    """
    Invalid JWTs should be rejected.
    """
    response = client.get("/api/v1/boundaries", headers={"Authorization": "Bearer invalid-token"})
    assert response.status_code == 401
    assert response.json()["detail"] == "Could not validate credentials"

def test_boundaries_valid_token(client: TestClient):
    """
    Tests that accessing a protected endpoint with a valid token succeeds.
    """
    token_payload = {
        "sub": TEST_USER_ID,
        "aud": "authenticated",
        "role": "user",
        "email": "test@example.com"
    }
    token = create_test_token(token_payload)
    
    response = client.get("/api/v1/boundaries", headers={"Authorization": f"Bearer {token}"})
    assert response.status_code == 200
    assert isinstance(response.json(), list)
