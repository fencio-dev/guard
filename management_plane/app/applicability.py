"""
Rule-family based applicability evaluator.

Deterministic, extensible applicability checks that avoid over-strict filtering.
Each rule returns a tri-state result (match, mismatch, abstain) with a weight.
Core rules must not mismatch; soft rules vote toward an applicability score.

Environment configuration (optional):
- APPLICABILITY_MODE: "soft" (default) or "strict"
- APPLICABILITY_MIN_SCORE: float in [0,1], default 0.5

No external dependencies; operates purely on Pydantic models from app.models.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Literal, Protocol

from pydantic import BaseModel, Field

from .models import IntentEvent, DesignBoundary


Decision = Literal["match", "mismatch", "abstain"]


class RuleOutcome(BaseModel):
    rule_id: str
    decision: Decision
    weight: float = Field(ge=0.0)
    reason: str


class ApplicabilityResult(BaseModel):
    applicable: bool
    score: float
    outcomes: list[RuleOutcome]


class ApplicabilityRule(Protocol):
    id: str
    weight: float
    kind: Literal["core", "soft"]

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome: ...


def _get_mode() -> Literal["soft", "strict"]:
    mode = os.getenv("APPLICABILITY_MODE", "soft").strip().lower()
    return "strict" if mode == "strict" else "soft"


def _get_min_score() -> float:
    try:
        return max(0.0, min(1.0, float(os.getenv("APPLICABILITY_MIN_SCORE", "0.5"))))
    except Exception:
        return 0.5


@dataclass(frozen=True)
class _ActionRule:
    id: str = "ActionRule"
    weight: float = 1.0
    kind: Literal["core", "soft"] = "core"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        actions = boundary.constraints.action.actions
        if intent.action in actions:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"action {intent.action} in {actions}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"action {intent.action} not in {actions}")


@dataclass(frozen=True)
class _ActorTypeRule:
    id: str = "ActorTypeRule"
    weight: float = 1.0
    kind: Literal["core", "soft"] = "core"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        actor_types = boundary.constraints.action.actor_types
        # Actor types are part of the v1.2 contract; treat as core
        if intent.actor.type in actor_types:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"actor {intent.actor.type} in {actor_types}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"actor {intent.actor.type} not in {actor_types}")


@dataclass(frozen=True)
class _ResourceTypeRule:
    id: str = "ResourceTypeRule"
    weight: float = 1.0
    kind: Literal["core", "soft"] = "core"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        types = boundary.constraints.resource.types
        if intent.resource.type in types:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"resource.type {intent.resource.type} in {types}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"resource.type {intent.resource.type} not in {types}")


@dataclass(frozen=True)
class _LocationRule:
    id: str = "LocationRule"
    weight: float = 0.5
    kind: Literal["core", "soft"] = "soft"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        locations = boundary.constraints.resource.locations
        if not locations:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="boundary has no location constraint")
        if not intent.resource.location:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="intent has no resource.location")
        if intent.resource.location in locations:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"location {intent.resource.location} in {locations}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"location {intent.resource.location} not in {locations}")


@dataclass(frozen=True)
class _PiiRule:
    id: str = "PiiRule"
    weight: float = 0.5
    kind: Literal["core", "soft"] = "soft"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        target = boundary.constraints.data.pii
        if target is None:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="boundary has no pii requirement")
        value = intent.data.pii
        if value is None:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="intent has no pii field")
        if value == target:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"pii == {target}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"pii != {target}")


@dataclass(frozen=True)
class _VolumeRule:
    id: str = "VolumeRule"
    weight: float = 0.5
    kind: Literal["core", "soft"] = "soft"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        target = boundary.constraints.data.volume
        if target is None:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="boundary has no volume requirement")
        value = intent.data.volume
        if value is None:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="intent has no volume field")
        if value == target:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"volume == {target}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"volume != {target}")


@dataclass(frozen=True)
class _DomainRule:
    id: str = "DomainRule"
    weight: float = 0.25
    kind: Literal["core", "soft"] = "soft"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        domains = boundary.scope.domains
        if not domains:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="no scope.domains constraint")
        if intent.resource.type in domains:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"resource.type {intent.resource.type} in scope.domains {domains}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"resource.type {intent.resource.type} not in scope.domains {domains}")


@dataclass(frozen=True)
class _ResourceNameRule:
    id: str = "ResourceNameRule"
    weight: float = 0.25
    kind: Literal["core", "soft"] = "soft"

    def evaluate(self, intent: IntentEvent, boundary: DesignBoundary) -> RuleOutcome:
        names = boundary.constraints.resource.names
        if not names:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="boundary has no resource.names constraint")
        if not intent.resource.name:
            return RuleOutcome(rule_id=self.id, decision="abstain", weight=self.weight, reason="intent has no resource.name")
        if intent.resource.name in names:
            return RuleOutcome(rule_id=self.id, decision="match", weight=self.weight, reason=f"resource.name {intent.resource.name} in {names}")
        return RuleOutcome(rule_id=self.id, decision="mismatch", weight=self.weight, reason=f"resource.name {intent.resource.name} not in {names}")


CORE_RULES: tuple[ApplicabilityRule, ...] = (
    _ActionRule(),
    _ActorTypeRule(),
    _ResourceTypeRule(),
)

SOFT_RULES: tuple[ApplicabilityRule, ...] = (
    _LocationRule(),
    _PiiRule(),
    _VolumeRule(),
    _DomainRule(),
    _ResourceNameRule(),
)


def evaluate_applicability(intent: IntentEvent, boundary: DesignBoundary) -> ApplicabilityResult:
    """
    Evaluate whether a boundary applies to an intent.

    - Core rules must not mismatch.
    - Soft rules produce a normalized score in [-1, 1] mapped to [0,1].
      We accept if score >= APPLICABILITY_MIN_SCORE (default 0.5).
    - If no soft rules participate (all abstain), score defaults to 1.0.
    """
    outcomes: list[RuleOutcome] = []

    # Core checks
    for rule in CORE_RULES:
        outcome = rule.evaluate(intent, boundary)
        outcomes.append(outcome)
        if outcome.decision == "mismatch":
            return ApplicabilityResult(applicable=False, score=0.0, outcomes=outcomes)

    # Soft votes
    num = 0.0
    den = 0.0
    for rule in SOFT_RULES:
        outcome = rule.evaluate(intent, boundary)
        outcomes.append(outcome)
        if outcome.decision == "abstain":
            continue
        if outcome.decision == "match":
            num += outcome.weight
        elif outcome.decision == "mismatch":
            num -= outcome.weight
        den += outcome.weight

    # Normalize to [0,1]
    if den == 0.0:
        score = 1.0  # no soft information → accept by default
    else:
        # num in [-den, +den] → map to [0,1]
        score = (num + den) / (2 * den)

    mode = _get_mode()
    min_score = _get_min_score()

    if mode == "strict":
        # In strict mode, require all participating soft rules to match
        participating = [o for o in outcomes if o.rule_id not in {r.id for r in CORE_RULES} and o.decision != "abstain"]
        if any(o.decision == "mismatch" for o in participating):
            return ApplicabilityResult(applicable=False, score=score, outcomes=outcomes)

    applicable = score >= min_score
    return ApplicabilityResult(applicable=applicable, score=score, outcomes=outcomes)


__all__ = [
    "RuleOutcome",
    "ApplicabilityResult",
    "evaluate_applicability",
]

