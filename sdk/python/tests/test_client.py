"""
Unit tests for TuplClient and AsyncTuplClient.

Tests cover:
- Client initialization
- Event creation and validation
- HTTP communication (mocked)
- Error handling
"""

import pytest
import time
import uuid
from unittest.mock import Mock, patch, MagicMock
import httpx

from tupl import TuplClient, AsyncTuplClient, IntentEvent, Actor, Resource, Data, Risk, ComparisonResult, DataPlaneError


# ============================================================================
# Test Fixtures
# ============================================================================

@pytest.fixture
def sample_intent_event():
    """Create a sample IntentEvent for testing."""
    return IntentEvent(
        id=f"evt-{uuid.uuid4()}",
        schemaVersion="v1.3",
        tenantId="tenant-test",
        timestamp=time.time(),
        actor=Actor(id="user-test", type="user"),
        action="read",
        resource=Resource(type="database", name="test_db", location="cloud"),
        data=Data(sensitivity=["internal"], pii=True, volume="single"),
        risk=Risk(authn="required")
    )


@pytest.fixture
def sample_comparison_result():
    """Create a sample ComparisonResult for testing."""
    return ComparisonResult(
        decision=1,
        slice_similarities=[0.92, 0.88, 0.85, 0.90]
    )


# ============================================================================
# TuplClient Tests
# ============================================================================

class TestTuplClient:
    """Tests for the synchronous TuplClient."""

    def test_client_initialization_default(self):
        """Test client initializes with default values."""
        client = TuplClient()

        assert client.endpoint == "http://localhost:8000"
        assert client.api_version == "v1"
        assert client.compare_url == "http://localhost:8000/api/v1/intents/compare"
        assert client.buffered is False
        assert client.buffer is None

        client.close()

    def test_client_initialization_custom(self):
        """Test client initializes with custom values."""
        client = TuplClient(
            endpoint="http://example.com:9000",
            api_version="v2",
            timeout=20.0
        )

        assert client.endpoint == "http://example.com:9000"
        assert client.api_version == "v2"
        assert client.compare_url == "http://example.com:9000/api/v2/intents/compare"
        assert client.timeout == 20.0

        client.close()

    def test_client_initialization_buffered(self):
        """Test client initializes in buffered mode."""
        client = TuplClient(
            buffered=True,
            buffer_size=20,
            buffer_timeout=10.0
        )

        assert client.buffered is True
        assert client.buffer is not None
        assert client.buffer.max_size == 20
        assert client.buffer.flush_interval == 10.0

        client.close()

    def test_client_endpoint_trailing_slash(self):
        """Test client strips trailing slash from endpoint."""
        client = TuplClient(endpoint="http://localhost:8000/")
        assert client.endpoint == "http://localhost:8000"
        client.close()

    @patch('httpx.Client.post')
    def test_capture_immediate_mode_success(self, mock_post, sample_intent_event, sample_comparison_result):
        """Test capturing an event in immediate mode (success case)."""
        # Mock HTTP response
        mock_response = Mock()
        mock_response.json.return_value = sample_comparison_result.model_dump()
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response

        # Create client and capture event
        client = TuplClient()
        result = client.capture(sample_intent_event)

        # Verify result
        assert result is not None
        assert isinstance(result, ComparisonResult)
        assert result.decision == 1
        assert len(result.slice_similarities) == 4

        # Verify HTTP call
        mock_post.assert_called_once()
        call_args = mock_post.call_args
        assert call_args[0][0] == "http://localhost:8000/api/v1/intents/compare"
        assert call_args[1]["headers"]["Content-Type"] == "application/json"

        client.close()

    @patch('httpx.Client.post')
    def test_capture_immediate_mode_http_error(self, mock_post, sample_intent_event):
        """Test capturing an event when HTTP request fails."""
        # Mock HTTP error
        mock_post.side_effect = httpx.HTTPError("Connection failed")

        # Create client and capture event
        client = TuplClient()
        result = client.capture(sample_intent_event)

        # Verify error handling
        assert result is None

        client.close()

    @patch('httpx.Client.post')
    def test_capture_immediate_mode_invalid_response(self, mock_post, sample_intent_event):
        """Test capturing an event when response is invalid."""
        # Mock invalid response
        mock_response = Mock()
        mock_response.json.return_value = {"invalid": "data"}
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response

        # Create client and capture event
        client = TuplClient()
        result = client.capture(sample_intent_event)

        # Verify error handling
        assert result is None

        client.close()

    def test_capture_buffered_mode(self, sample_intent_event):
        """Test buffered mode configuration."""
        # Create client in buffered mode
        client = TuplClient(
            buffered=True,
            buffer_size=10,
            buffer_timeout=5.0
        )

        # Verify buffered mode is enabled
        assert client.buffered is True
        assert client.buffer is not None
        assert client.buffer.max_size == 10
        assert client.buffer.flush_interval == 5.0

        client.close()

    def test_context_manager(self):
        """Test client works as context manager."""
        with TuplClient() as client:
            assert client.endpoint == "http://localhost:8000"

        # Client should be closed after context

    def test_flush_in_immediate_mode(self):
        """Test flush() in immediate mode (should be no-op)."""
        client = TuplClient(buffered=False)

        # Should not raise error
        client.flush()

        client.close()

    @patch('httpx.Client.post')
    def test_enforce_intent_success(self, mock_post, sample_intent_event, sample_comparison_result):
        mock_response = Mock()
        mock_response.json.return_value = sample_comparison_result.model_dump()
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response

        client = TuplClient()
        result = client.enforce_intent(sample_intent_event)

        assert isinstance(result, ComparisonResult)
        mock_post.assert_called_once()
        assert mock_post.call_args[0][0] == "http://localhost:8000/api/v1/enforce"

        client.close()

    @patch('httpx.Client.post')
    def test_enforce_intent_http_error(self, mock_post, sample_intent_event):
        mock_post.side_effect = httpx.HTTPError("boom")

        client = TuplClient()
        with pytest.raises(DataPlaneError):
            client.enforce_intent(sample_intent_event)

        client.close()


# ============================================================================
# AsyncTuplClient Tests
# ============================================================================

class TestAsyncTuplClient:
    """Tests for the asynchronous AsyncTuplClient."""

    @pytest.mark.asyncio
    async def test_async_client_initialization(self):
        """Test async client initializes correctly."""
        client = AsyncTuplClient()

        assert client.endpoint == "http://localhost:8000"
        assert client.api_version == "v1"
        assert client.compare_url == "http://localhost:8000/api/v1/intents/compare"

        await client.close()

    @pytest.mark.asyncio
    async def test_async_client_context_manager(self):
        """Test async client works as context manager."""
        async with AsyncTuplClient() as client:
            assert client.endpoint == "http://localhost:8000"

        # Client should be closed after context

    @pytest.mark.asyncio
    @patch('httpx.AsyncClient.post')
    async def test_async_capture_success(self, mock_post, sample_intent_event, sample_comparison_result):
        """Test async capturing an event (success case)."""
        # Mock HTTP response
        mock_response = Mock()
        mock_response.json.return_value = sample_comparison_result.model_dump()
        mock_response.raise_for_status = Mock()
        mock_post.return_value = mock_response

        # Create client and capture event
        client = AsyncTuplClient()
        result = await client.capture(sample_intent_event)

        # Verify result
        assert result is not None
        assert isinstance(result, ComparisonResult)
        assert result.decision == 1

        await client.close()


# ============================================================================
# IntentEvent Validation Tests
# ============================================================================

class TestIntentEventValidation:
    """Tests for IntentEvent validation."""

    def test_create_valid_intent_event(self):
        """Test creating a valid IntentEvent."""
        event = IntentEvent(
            id="evt-001",
            tenantId="tenant-123",
            timestamp=time.time(),
            actor=Actor(id="user-123", type="user"),
            action="read",
            resource=Resource(type="database", name="users_db"),
            data=Data(sensitivity=["internal"], pii=True, volume="single"),
            risk=Risk(authn="required")
        )

        assert event.id == "evt-001"
        assert event.schemaVersion == "v1"
        assert event.action == "read"
        assert event.actor.type == "user"

    def test_intent_event_invalid_action(self):
        """Test IntentEvent with invalid action raises error."""
        with pytest.raises(ValueError):
            IntentEvent(
                id="evt-001",
                tenantId="tenant-123",
                timestamp=time.time(),
                actor=Actor(id="user-123", type="user"),
                action="invalid_action",  # Invalid
                resource=Resource(type="database"),
                data=Data(sensitivity=["internal"], pii=True, volume="single"),
                risk=Risk(authn="required")
            )

    def test_intent_event_serialization(self, sample_intent_event):
        """Test IntentEvent serializes to JSON correctly."""
        json_data = sample_intent_event.model_dump(mode="json")

        assert "id" in json_data
        assert "schemaVersion" in json_data
        assert "actor" in json_data
        assert json_data["schemaVersion"] == "v1"

    def test_comparison_result_validation(self):
        """Test ComparisonResult validation."""
        # Valid result
        result = ComparisonResult(
            decision=1,
            slice_similarities=[0.9, 0.8, 0.7, 0.6]
        )
        assert result.decision == 1

        # Invalid decision (out of range)
        with pytest.raises(ValueError):
            ComparisonResult(decision=2, slice_similarities=[0.9, 0.8, 0.7, 0.6])

        # Invalid similarities (wrong length)
        with pytest.raises(ValueError):
            ComparisonResult(decision=1, slice_similarities=[0.9, 0.8])
