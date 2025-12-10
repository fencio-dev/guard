"""Canonical vocabulary registry for semantic security encoding."""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path
from typing import Any, Literal

import yaml

SlotName = Literal["action", "resource", "data", "risk"]


class VocabularyRegistry:
    """Singleton loader for the canonical vocabulary YAML."""

    _instance: "VocabularyRegistry" | None = None

    def __new__(cls) -> "VocabularyRegistry":
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._load_vocabulary()
        return cls._instance

    def _load_vocabulary(self) -> None:
        """Load `vocabulary.yaml` from the package directory."""
        # First try loading from package directory (installed package)
        vocab_path = Path(__file__).parent / "vocabulary.yaml"

        # Fallback to monorepo root (development mode)
        if not vocab_path.exists():
            vocab_path = Path(__file__).resolve().parents[3] / "vocabulary.yaml"

        if not vocab_path.exists():
            raise FileNotFoundError(f"Vocabulary file not found: {vocab_path}")

        with vocab_path.open("r", encoding="utf-8") as handle:
            self._vocab = yaml.safe_load(handle)

    # ------------------------------------------------------------------
    # Metadata
    # ------------------------------------------------------------------

    def get_version(self) -> str:
        """Return the declared vocabulary version."""
        return self._vocab.get("version", "unknown")

    def get_metadata(self) -> dict[str, Any]:
        """Return metadata block (description, last_updated)."""
        return self._vocab.get("metadata", {})

    # ------------------------------------------------------------------
    # Vocabulary lookups
    # ------------------------------------------------------------------

    def get_valid_actions(self) -> list[str]:
        """Return canonical action values."""
        return list(self._vocab.get("vocabulary", {}).get("actions", {}).keys())

    def get_valid_resource_types(self) -> list[str]:
        """Return canonical resource types."""
        return list(self._vocab.get("vocabulary", {}).get("resource_types", {}).keys())

    def get_sensitivity_levels(self) -> list[str]:
        """Return canonical sensitivity levels."""
        return list(self._vocab.get("vocabulary", {}).get("sensitivity_levels", {}).keys())

    def get_volumes(self) -> list[str]:
        """Return canonical volume values."""
        return list(self._vocab.get("vocabulary", {}).get("volumes", {}).keys())

    def get_authn_levels(self) -> list[str]:
        """Return canonical authentication level values."""
        return list(self._vocab.get("vocabulary", {}).get("authn_levels", {}).keys())

    def get_params_length_buckets(self) -> list[str]:
        """Return canonical params_length bucket values."""
        return list(self._vocab.get("vocabulary", {}).get("params_length_buckets", {}).keys())

    def get_action_keywords(self, action: str) -> list[str]:
        """Return keywords that map to a canonical action."""
        return self._vocab["vocabulary"]["actions"].get(action, {}).get("keywords", [])

    def map_keyword_to_action(self, keyword: str) -> str | None:
        """Map a keyword to its canonical action if present."""
        keyword_lower = keyword.lower()
        for action, config in self._vocab["vocabulary"]["actions"].items():
            keywords = config.get("keywords", [])
            if keyword_lower in keywords:
                return action
        return None

    def infer_action_from_tool_name(self, tool_name: str) -> str:
        """Infer canonical action from the tool name using vocabulary keywords."""
        normalized = tool_name.replace("-", " ").replace("_", " ")
        for part in normalized.split():
            action = self.map_keyword_to_action(part)
            if action:
                return action
        return "execute"

    def infer_resource_type_from_tool_name(self, tool_name: str) -> str:
        """Infer resource type from the tool name using vocabulary keywords."""
        tool_lower = tool_name.lower()
        resource_types = self._vocab["vocabulary"]["resource_types"]
        for res_type, config in resource_types.items():
            keywords = config.get("keywords", [])
            if any(kw in tool_lower for kw in keywords):
                return res_type
        return "api"

    # ------------------------------------------------------------------
    # Template assembly
    # ------------------------------------------------------------------

    def assemble_anchor(self, slot: SlotName, fields: dict[str, Any]) -> str:
        """Assemble slot anchor string by selecting the appropriate template."""
        templates = self._vocab.get("templates", {}).get(slot, {})
        template = templates.get("base") if slot != "action" else templates.get("format")

        if slot == "action":
            template = templates.get("with_tool_call") if "tool_call" in fields else templates.get("format")
        elif slot == "resource":
            has_location = "resource_location" in fields
            has_name = "resource_name" in fields
            has_tool = "tool_name" in fields and "tool_method" in fields
            if has_tool and has_location and has_name:
                template = templates.get("full")
            elif has_tool:
                template = templates.get("with_tool")
            elif has_name:
                template = templates.get("with_name")
            elif has_location:
                template = templates.get("with_location")
            else:
                template = templates.get("minimal")
        elif slot == "data":
            template = templates.get("with_params") if "params_length" in fields else templates.get("base")
        elif slot == "risk":
            template = templates.get("with_rate_limit") if "rate_limit" in fields else templates.get("base")

        if template is None:
            raise ValueError(f"Template not defined for slot {slot}")

        return template.format(**fields)

    # ------------------------------------------------------------------
    # Extraction helpers
    # ------------------------------------------------------------------

    def get_extraction_rules(self, family_id: str) -> dict[str, Any]:
        """Return extraction rules for the specified family."""
        return self._vocab.get("extraction_rules", {}).get(family_id, {})

    def get_examples(self, family_id: str) -> dict[str, Any]:
        """Return example expectations for the family."""
        return self._vocab.get("examples", {}).get(family_id, {})

    # ------------------------------------------------------------------
    # Validation helpers
    # ------------------------------------------------------------------

    def is_valid_action(self, action: str) -> bool:
        """Return True if action is in the vocabulary."""
        return action in self.get_valid_actions()

    def is_valid_resource_type(self, resource_type: str) -> bool:
        """Return True if resource type is in the vocabulary."""
        return resource_type in self.get_valid_resource_types()
