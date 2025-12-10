"""Tests for the shared vocabulary registry."""

from tupl.vocabulary import VocabularyRegistry


def test_registry_keywords_and_templates() -> None:
    vocab = VocabularyRegistry()

    assert vocab.get_version() == "1.0"
    assert "read" in vocab.get_valid_actions()

    assert vocab.infer_action_from_tool_name("search_database") == "read"
    assert vocab.infer_resource_type_from_tool_name("search_database") == "database"

    action_anchor = vocab.assemble_anchor(
        "action",
        {"action": "read", "actor_type": "agent", "tool_call": "search_database.read"},
    )
    assert action_anchor == "action is read | actor_type is agent | tool_call is search_database.read"

    data_anchor = vocab.assemble_anchor(
        "data",
        {"sensitivity": "public", "pii": False, "volume": "single", "params_length": "short"},
    )
    assert data_anchor == "sensitivity is public | pii is False | volume is single | params_length is short"
