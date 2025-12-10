# app/nl_policy_parser.py
from typing import Optional, Literal
from pydantic import BaseModel
import hashlib
import logging
from google import genai
from google.genai import types

logger = logging.getLogger(__name__)

class ActionConstraints(BaseModel):
    actions: list[str]
    actor_types: list[str]

class ResourceConstraints(BaseModel):
    types: list[str]
    names: Optional[list[str]] = None
    locations: Optional[list[str]] = None

class DataConstraints(BaseModel):
    sensitivity: list[str]
    pii: Optional[bool] = None
    volume: Optional[str] = None

class RiskConstraints(BaseModel):
    authn: str

class PolicyConstraints(BaseModel):
    action: ActionConstraints
    resource: ResourceConstraints
    data: DataConstraints
    risk: RiskConstraints

class SliceThresholds(BaseModel):
    action: float = 0.5
    resource: float = 0.5
    data: float = 0.5
    risk: float = 0.5

# ==============================================================================
# THRESHOLD PRIORITY DOCUMENTATION
# ==============================================================================
#
# The SliceThresholds defined above (lines 35-38) are the canonical default
# values for policy generation. These defaults are prioritized over any
# hardcoded test values in the Rust comparison logic.
#
# Architecture:
# -------------
# 1. Python Layer (nl_policy_parser.py):
#    - Defines default thresholds: action=0.50, resource=0.50, data=0.50, risk=0.50
#    - Used when generating policies from natural language
#    - Embedded in LLM prompt (line 89) as recommended values
#    - Priority order: action > resource > data > risk
#
# 2. Rust Layer (semantic-sandbox/src/compare.rs):
#    - Test cases use hardcoded example values (e.g., [0.85, 0.85, 0.85, 0.85])
#    - These are for testing comparison logic only, not policy defaults
#    - Actual runtime thresholds come from VectorEnvelope populated by this system
#
# Data Flow:
# ----------
#   Natural Language Policy
#           ↓
#   NLPolicyParser.parse_policy()
#           ↓
#   SliceThresholds (0.50, 0.50, 0.50, 0.50)
#           ↓
#   VectorEnvelope.thresholds
#           ↓
#   Rust compare() function (semantic-sandbox)
#
# Summary: Python defaults are canonical. Rust executes whatever it receives.
# ==============================================================================

class PolicyRules(BaseModel):
    """Structured policy rules generated from natural language."""
    thresholds: SliceThresholds
    decision: Literal["min", "weighted-avg"] = "min"
    globalThreshold: Optional[float] = None
    constraints: PolicyConstraints

# Vocabulary constants (reference existing vocabulary.yaml)
VALID_ACTIONS = ["read", "write", "delete", "export", "execute"]
VALID_RESOURCE_TYPES = ["database", "file", "api"]
VALID_SENSITIVITY = ["public", "internal", "confidential"]
VALID_VOLUMES = ["single", "bulk"]
VALID_AUTHN = ["required", "not_required"]
VALID_ACTOR_TYPES = ["user", "agent", "service"]

class NLPolicyParser:
    """Parse natural language policy templates into structured PolicyRules."""

    def __init__(self, api_key: str):
        self.client = genai.Client(api_key=api_key)
        self.model = "gemini-2.0-flash-lite"
        self._cache: dict[str, PolicyRules] = {}

    def _compute_cache_key(self, template_id: str, customization: Optional[str]) -> str:
        """Compute cache key from template and customization."""
        content = f"{template_id}:{customization or ''}"
        return hashlib.sha256(content.encode()).hexdigest()

    def _build_prompt(self, template_text: str, customization: Optional[str]) -> str:
        """Build LLM prompt for policy generation."""
        return f"""You are a security policy generator for an AI agent guardrail system.

INPUT:
Template: {template_text}
Customization: {customization or "none"}

CANONICAL VOCABULARY:
Actions: {VALID_ACTIONS}
Resource Types: {VALID_RESOURCE_TYPES}
Sensitivity Levels: {VALID_SENSITIVITY}
Volumes: {VALID_VOLUMES}
Authn Levels: {VALID_AUTHN}
Actor Types: {VALID_ACTOR_TYPES}

TASK:
Generate a PolicyRules object that represents an ALLOW policy for this guardrail.

RULES:
1. Use ONLY vocabulary values listed above
2. Set default thresholds: action=0.50, resource=0.50, data=0.50, risk=0.50
3. Use "min" decision mode by default
4. Extract constraints from the natural language
5. For actor_types, default to ["user", "agent"] if not specified
6. For authn, default to "required" if not specified

OUTPUT: Return JSON matching the PolicyRules schema.
"""

    def _validate_vocabulary_compliance(self, policy: PolicyRules) -> None:
        """Validate that policy uses only canonical vocabulary."""
        # Validate actions
        for action in policy.constraints.action.actions:
            if action not in VALID_ACTIONS:
                raise ValueError(f"Invalid action '{action}'. Must be one of: {VALID_ACTIONS}")

        # Validate actor_types
        for actor_type in policy.constraints.action.actor_types:
            if actor_type not in VALID_ACTOR_TYPES:
                raise ValueError(f"Invalid actor_type '{actor_type}'. Must be one of: {VALID_ACTOR_TYPES}")

        # Validate resource types
        for rtype in policy.constraints.resource.types:
            if rtype not in VALID_RESOURCE_TYPES:
                raise ValueError(f"Invalid resource type '{rtype}'. Must be one of: {VALID_RESOURCE_TYPES}")

        # Validate sensitivity
        for sens in policy.constraints.data.sensitivity:
            if sens not in VALID_SENSITIVITY:
                raise ValueError(f"Invalid sensitivity '{sens}'. Must be one of: {VALID_SENSITIVITY}")

        # Validate volume if present
        if policy.constraints.data.volume and policy.constraints.data.volume not in VALID_VOLUMES:
            raise ValueError(f"Invalid volume '{policy.constraints.data.volume}'. Must be one of: {VALID_VOLUMES}")

        # Validate authn
        if policy.constraints.risk.authn not in VALID_AUTHN:
            raise ValueError(f"Invalid authn '{policy.constraints.risk.authn}'. Must be one of: {VALID_AUTHN}")

    async def parse_policy(
        self,
        template_id: str,
        template_text: str,
        customization: Optional[str]
    ) -> PolicyRules:
        """Parse natural language template into PolicyRules."""
        # Check cache
        cache_key = self._compute_cache_key(template_id, customization)
        if cache_key in self._cache:
            logger.info(f"Cache hit for template '{template_id}'")
            return self._cache[cache_key]

        # Build prompt
        prompt = self._build_prompt(template_text, customization)

        # Call Gemini with structured output
        try:
            response = self.client.models.generate_content(
                model=self.model,
                contents=prompt,
                config=types.GenerateContentConfig(
                    response_mime_type="application/json",
                    response_schema=PolicyRules,
                    temperature=0.3,
                ),
            )

            # Parse and validate
            policy_rules = PolicyRules.model_validate_json(response.text)

            # Validate vocabulary compliance
            self._validate_vocabulary_compliance(policy_rules)

            # Cache and return
            self._cache[cache_key] = policy_rules
            logger.info(f"Successfully parsed policy for template '{template_id}'")
            return policy_rules

        except Exception as e:
            logger.error(f"Failed to parse policy: {e}")
            raise ValueError(
                "Failed to parse policy from natural language. "
                "Please try rephrasing your customization or use a different template."
            ) from e
