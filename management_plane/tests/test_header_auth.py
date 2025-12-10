"""Tests for header-based authentication."""

import pytest
from fastapi import HTTPException
from app.auth import get_current_user_from_headers


def test_get_current_user_from_headers_success():
    """Should extract tenant from X-Tenant-Id header."""
    user = get_current_user_from_headers(
        x_tenant_id="550e8400-e29b-41d4-a716-446655440000",
        x_user_id=None
    )
    assert user.id == "550e8400-e29b-41d4-a716-446655440000"
    assert user.aud == "internal-header"
    assert user.role == "authenticated"


def test_get_current_user_from_headers_with_user_id():
    """Should support optional X-User-Id header."""
    user = get_current_user_from_headers(
        x_tenant_id="550e8400-e29b-41d4-a716-446655440000",
        x_user_id="660e8400-e29b-41d4-a716-446655440000"
    )
    assert user.id == "550e8400-e29b-41d4-a716-446655440000"
    assert user.email == "660e8400-e29b-41d4-a716-446655440000"  # Store user_id in email field


def test_get_current_user_from_headers_missing_tenant_id():
    """Should raise 401 if X-Tenant-Id missing."""
    with pytest.raises(HTTPException) as exc:
        get_current_user_from_headers(x_tenant_id=None, x_user_id=None)
    assert exc.value.status_code == 401
