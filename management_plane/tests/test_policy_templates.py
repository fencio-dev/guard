# tests/test_policy_templates.py
import pytest
from app.policy_templates import (
    PolicyTemplate,
    POLICY_TEMPLATES,
    get_template_by_id,
    get_templates_by_category
)

def test_policy_template_structure():
    """Test that all templates have required fields."""
    assert len(POLICY_TEMPLATES) > 0

    for template in POLICY_TEMPLATES:
        assert template.id
        assert template.name
        assert template.description
        assert template.template_text
        assert template.category in ["database", "file", "api", "general"]
        assert isinstance(template.example_customizations, list)

def test_template_ids_unique():
    """Test that template IDs are unique."""
    ids = [t.id for t in POLICY_TEMPLATES]
    assert len(ids) == len(set(ids))

def test_get_template_by_id_found():
    """Test retrieving an existing template by ID."""
    template = get_template_by_id("database_read_only")
    assert template is not None
    assert template.id == "database_read_only"
    assert template.name == "Database Read-Only Access"
    assert template.category == "database"

def test_get_template_by_id_not_found():
    """Test retrieving a non-existent template by ID."""
    template = get_template_by_id("nonexistent_template")
    assert template is None

def test_get_templates_by_category():
    """Test retrieving templates by category."""
    database_templates = get_templates_by_category("database")
    assert len(database_templates) == 2
    assert all(t.category == "database" for t in database_templates)
    assert any(t.id == "database_read_only" for t in database_templates)
    assert any(t.id == "database_write_access" for t in database_templates)

def test_get_templates_by_category_empty():
    """Test retrieving templates for a category with no templates."""
    empty_templates = get_templates_by_category("nonexistent_category")
    assert len(empty_templates) == 0
    assert empty_templates == []
