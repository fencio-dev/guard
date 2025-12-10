import json
import pytest

from app.llm_anchor_generator import AnchorSlots, LLMAnchorGenerator
from app.rule_encoding import build_rule_anchors


class DummyGenerator:
    def __init__(self, anchors: AnchorSlots):
        self.anchors = anchors
        self.call_args: list[tuple[dict, str]] = []

    async def generate_rule_anchors(self, rule: dict, family_id: str) -> AnchorSlots:
        self.call_args.append((rule, family_id))
        return self.anchors


@pytest.mark.asyncio
async def test_build_rule_anchors_uses_llm(monkeypatch):
    rule = {"rule_id": "rule_123", "allowed_tool_ids": ["foo"]}
    anchor_slots = AnchorSlots(
        action=["act"],
        resource=["res"],
        data=["dat"],
        risk=["risk"],
    )

    dummy_generator = DummyGenerator(anchor_slots)

    def fake_get_generator():
        return dummy_generator

    def fake_encode(anchors, slot_name, seed, max_anchors=16):
        encoded = [[float(seed)] * 32 for _ in anchors]
        while len(encoded) < 16:
            encoded.append([0.0] * 32)
        return encoded, len(anchors)

    monkeypatch.setattr("app.rule_encoding.get_llm_generator", fake_get_generator)
    monkeypatch.setattr("app.rule_encoding.encode_anchor_list", fake_encode)

    result = await build_rule_anchors(rule, "tool_whitelist")

    assert result["action_count"] == 1
    assert result["resource_count"] == 1
    assert result["data_count"] == 1
    assert result["risk_count"] == 1
    assert dummy_generator.call_args == [(rule, "tool_whitelist")]


@pytest.mark.asyncio
async def test_llm_anchor_generator_assembles_vocabulary_templates(monkeypatch):
    class DummyResponse:
        def __init__(self, text: str):
            self.text = text

    class DummyModels:
        def generate_content(self, *args, **kwargs):
            payload = json.dumps(
                {
                    "action": [
                        {"action": "read", "actor_type": "agent", "tool_call": "search_database.read"}
                    ],
                    "resource": [
                        {
                            "resource_type": "database",
                            "resource_location": "cloud",
                            "resource_name": "search_database",
                            "tool_name": "search_database",
                            "tool_method": "read",
                        }
                    ],
                    "data": [
                        {
                            "sensitivity": "public",
                            "pii": False,
                            "volume": "single",
                            "params_length": "short",
                        }
                    ],
                    "risk": [{"authn": "required"}],
                }
            )
            return DummyResponse(payload)

    class DummyClient:
        def __init__(self, api_key: str):
            self.models = DummyModels()

    monkeypatch.setattr("app.llm_anchor_generator.genai.Client", DummyClient)

    generator = LLMAnchorGenerator(api_key="test-key")
    anchors = await generator.generate_rule_anchors({"rule_id": "dummy"}, "tool_whitelist")

    assert anchors.action == ["action is read | actor_type is agent | tool_call is search_database.read"]
    assert anchors.resource == [
        "resource_type is database | resource_location is cloud | resource_name is search_database | tool_name is search_database | tool_method is read"
    ]
    assert anchors.data == ["sensitivity is public | pii is False | volume is single | params_length is short"]
    assert anchors.risk == ["authn is required"]
