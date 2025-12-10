# tests/test_soft_block.py
import pytest
from unittest.mock import Mock, patch
from tupl.agent import SecureGraphProxy

def test_soft_block_logs_without_raising():
    """Test that soft-block mode logs violations without raising."""
    mock_graph = Mock()
    mock_event = Mock(tool_name="execute_query", id="event-123")
    mock_result = Mock(decision=0, slice_similarities={"action": 0.5})

    with patch('httpx.post'):  # Mock registration
        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            soft_block=True
        )

    # Should not raise
    with patch('logging.Logger.warning') as mock_log:
        proxy._handle_block_decision(mock_event, mock_result)

        # Verify warning was logged
        mock_log.assert_called_once()
        assert "SOFT-BLOCK" in str(mock_log.call_args)

def test_hard_block_raises():
    """Test that hard-block mode raises on violations."""
    mock_graph = Mock()
    mock_event = Mock(tool_name="execute_query", id="event-123")
    mock_result = Mock(decision=0, slice_similarities={"action": 0.5})

    with patch('httpx.post'):  # Mock registration
        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            soft_block=False  # Explicitly force hard-block behavior
        )

    with pytest.raises(PermissionError):
        proxy._handle_block_decision(mock_event, mock_result)

def test_custom_soft_block_handler():
    """Test custom soft-block handler."""
    mock_graph = Mock()
    mock_event = Mock()
    mock_result = Mock(decision=0)

    custom_handler = Mock()

    with patch('httpx.post'):  # Mock registration
        proxy = SecureGraphProxy(
            graph=mock_graph,
            agent_id="test-agent",
            boundary_id="test-boundary",
            soft_block=True,
            on_soft_block=custom_handler
        )

    proxy._handle_block_decision(mock_event, mock_result)

    # Verify custom handler was called
    custom_handler.assert_called_once_with(mock_event, mock_result)
