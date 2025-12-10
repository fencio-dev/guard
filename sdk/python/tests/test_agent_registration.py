# tests/test_agent_registration.py
import pytest
from unittest.mock import Mock, patch
from tupl.agent import SecureGraphProxy

def test_agent_registration_on_init():
    """Test that agent auto-registers on initialization."""
    mock_graph = Mock()

    with patch('httpx.post') as mock_post:
        mock_post.return_value.status_code = 200

        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            tenant_id="tenant-123",
            token="test-token",
            base_url="http://localhost:8000"
        )

        # Verify registration was called
        mock_post.assert_called_once()
        call_args = mock_post.call_args
        assert "/api/v1/agents/register" in call_args[0][0]
        assert call_args[1]["json"]["agent_id"] == "test-agent"

def test_agent_registration_failure_non_critical():
    """Test that registration failures don't break initialization."""
    mock_graph = Mock()

    with patch('httpx.post') as mock_post:
        mock_post.side_effect = Exception("Network error")

        # Should not raise
        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            tenant_id="tenant-123",
            token="test-token"
        )

        assert proxy.agent_id == "test-agent"
