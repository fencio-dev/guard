"""
Intent comparison endpoint.

Handles POST /api/v1/intents/compare for comparing IntentEvents against
DesignBoundaries.
"""

import logging
import time
from typing import Annotated

from fastapi import APIRouter, Body, Depends, HTTPException, status

from ..auth import User, get_current_user
from ..models import (
    ComparisonResult,
    BoundaryEvidence,
    IntentEvent,
    DesignBoundary,
    BoundaryScope,
    BoundaryRules,
    SliceThresholds,
    SliceWeights,
    BoundaryConstraints,
    ActionConstraint,
    ResourceConstraint,
    DataConstraint,
    RiskConstraint,
)
from ..encoding import encode_to_128d, encode_boundary_to_128d_cached
from ..applicability import evaluate_applicability, ApplicabilityResult
from ..ffi_bridge import get_sandbox
from .boundaries import _boundaries_store

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/intents", tags=["intents"])


def is_boundary_applicable(intent: IntentEvent, boundary: DesignBoundary) -> bool:
    """
    Backward-compatible wrapper that delegates to rule-family evaluator.

    Logs applicability score and top reasons. Returns only the boolean flag
    for use in simple list comprehensions. Detailed evidence is available
    from evaluate_applicability for Phase B.
    """
    result: ApplicabilityResult = evaluate_applicability(intent, boundary)
    # Build a concise reason string for logs
    reasons = ", ".join(
        f"{o.rule_id}:{o.decision}" for o in result.outcomes if o.decision != "abstain"
    )
    logger.debug(
        f"Applicability for boundary '{boundary.name}' → {result.applicable} "
        f"(score={result.score:.2f}; {reasons or 'no soft rules'})"
    )
    return result.applicable


# Week 2: Create a test boundary for demonstration (v1.1 with constraints)
# Week 3: This will be loaded from database
def _get_test_boundary() -> DesignBoundary:
    """Create a test boundary for Week 2 demonstration (v1.1)."""
    return DesignBoundary(
        id="test_boundary_001",
        name="Safe Read Access",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1.1",
        scope=BoundaryScope(
            tenantId="tenant_test",
            domains=["database", "api"],
        ),
        rules=BoundaryRules(
            thresholds=SliceThresholds(
                action=0.8,
                resource=0.75,
                data=0.7,
                risk=0.6,
            ),
            weights=SliceWeights(
                action=1.0,
                resource=1.0,
                data=1.0,
                risk=1.0,
            ),
            decision="min",
            globalThreshold=0.75,
        ),
        constraints=BoundaryConstraints(
            action=ActionConstraint(
                actions=["read"],
                actor_types=["user"],
            ),
            resource=ResourceConstraint(
                types=["database"],
                locations=["cloud"],
            ),
            data=DataConstraint(
                sensitivity=["internal"],
                pii=False,
                volume="single",
            ),
            risk=RiskConstraint(
                authn="required",
            ),
        ),
        notes="Test boundary for Week 2 (v1.1) - allows read operations on databases",
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )


@router.post("/compare", response_model=ComparisonResult, status_code=status.HTTP_200_OK)
async def compare_intent(
    event: Annotated[IntentEvent, Body(..., description="Intent event to compare against boundaries")],
    current_user: User = Depends(get_current_user)
) -> ComparisonResult:
    """
    Compare an intent event against design boundaries.

    Loads ALL active boundaries from storage and compares against each.
    Implements aggregation logic per plan.md section 3.4:
    - Mandatory boundaries: ALL must pass (min mode)
    - Optional boundaries: Weighted average
    - Final decision: ALLOW only if all mandatory boundaries pass

    Args:
        event: IntentEvent to evaluate

    Returns:
        ComparisonResult with decision and similarity scores

    Example:
        ```json
        POST /api/v1/intents/compare
        {
          "id": "evt_123",
          "schemaVersion": "v1",
          "timestamp": "2025-11-12T12:00:00Z",
          "actor": {
            "id": "user_456",
            "type": "human",
            "trustLevel": "verified"
          },
          "action": "read",
          "resource": {
            "type": "database",
            "identifier": "users_table",
            "sensitivity": "high"
          },
          "data": {
            "volumeClass": "single",
            "containsPII": true,
            "dataClassification": "confidential"
          },
          "risk": {
            "mutability": "readonly",
            "scope": "internal",
            "reversibility": "reversible"
          },
          "context": {
            "sessionId": "sess_789",
            "purpose": "User profile lookup"
          }
        }
        ```

        Response:
        ```json
        {
          "decision": 1,
          "slice_similarities": [0.92, 0.88, 0.85, 0.90]
        }
        ```
    """
    try:
        logger.info(f"Comparing intent: {event.id} - {event.action} on {event.resource.type}")
        logger.debug(f"Full event: {event.model_dump_json()}")

        # Encode the intent event to 128-dim vector
        intent_vector = encode_to_128d(event)
        logger.debug(f"Encoded intent to 128-dim vector (norm={intent_vector.sum():.4f})")

        # Load ALL active boundaries from storage
        # Filter by tenant if needed (for MVP, we use all boundaries)
        active_boundaries = [
            b for b in _boundaries_store.values()
            if b.status == "active"
        ]

        if not active_boundaries:
            # No boundaries configured - default to ALLOW with warning
            logger.warning(
                f"No active boundaries found for intent {event.id}. "
                "Defaulting to ALLOW (no policy enforcement)."
            )
            return ComparisonResult(
                decision=1,  # ALLOW
                slice_similarities=[1.0, 1.0, 1.0, 1.0],  # Perfect scores
            )

        logger.info(f"Loaded {len(active_boundaries)} active boundaries for comparison")

        # Filter to only applicable boundaries (Phase 2.1: Applicability Filter)
        applicable_boundaries = [
            b for b in active_boundaries
            if is_boundary_applicable(event, b)
        ]

        logger.info(
            f"Filtered to {len(applicable_boundaries)} applicable boundaries "
            f"from {len(active_boundaries)} total"
        )

        if not applicable_boundaries:
            # No applicable boundaries - default to BLOCK (fail-closed security)
            # (This is a policy decision: if no policies apply, deny by default)
            logger.warning(
                f"No applicable boundaries for intent {event.id}. "
                "Defaulting to BLOCK (fail-closed security)."
            )
            return ComparisonResult(
                decision=0,  # BLOCK
                slice_similarities=[0.0, 0.0, 0.0, 0.0],  # Zero scores
            )

        # Compare against each applicable boundary
        sandbox = get_sandbox()
        boundary_results: list[tuple[DesignBoundary, int, list[float]]] = []
        evidence: list[BoundaryEvidence] = []

        for boundary in applicable_boundaries:
            # Generate anchor embeddings per slot (Phase 2)
            from app.encoding import (
                build_boundary_action_anchors,
                build_boundary_resource_anchors,
                build_boundary_data_anchors,
                build_boundary_risk_anchors,
                encode_anchors_to_32d,
            )

            action_anchors, action_count = encode_anchors_to_32d(
                build_boundary_action_anchors(boundary),
                slot_name="action",
                slot_seed=42,
            )
            resource_anchors, resource_count = encode_anchors_to_32d(
                build_boundary_resource_anchors(boundary),
                slot_name="resource",
                slot_seed=43,
            )
            data_anchors, data_count = encode_anchors_to_32d(
                build_boundary_data_anchors(boundary),
                slot_name="data",
                slot_seed=44,
            )
            risk_anchors, risk_count = encode_anchors_to_32d(
                build_boundary_risk_anchors(boundary),
                slot_name="risk",
                slot_seed=45,
            )

            # Prepare comparison parameters
            thresholds = [
                boundary.rules.thresholds.action,
                boundary.rules.thresholds.resource,
                boundary.rules.thresholds.data,
                boundary.rules.thresholds.risk,
            ]

            weights = [
                boundary.rules.weights.action if boundary.rules.weights else 1.0,
                boundary.rules.weights.resource if boundary.rules.weights else 1.0,
                boundary.rules.weights.data if boundary.rules.weights else 1.0,
                boundary.rules.weights.risk if boundary.rules.weights else 1.0,
            ]

            decision_mode = 0 if boundary.rules.decision == "min" else 1
            global_threshold = boundary.rules.globalThreshold or 0.75

            # Call Rust sandbox with anchor arrays (Phase 2)
            decision, similarities = sandbox.compare(
                intent_vector=intent_vector,
                action_anchors=action_anchors,
                action_anchor_count=action_count,
                resource_anchors=resource_anchors,
                resource_anchor_count=resource_count,
                data_anchors=data_anchors,
                data_anchor_count=data_count,
                risk_anchors=risk_anchors,
                risk_anchor_count=risk_count,
                thresholds=thresholds,
                weights=weights,
                decision_mode=decision_mode,
                global_threshold=global_threshold,
            )

            boundary_results.append((boundary, decision, similarities))

            # Build evidence for this boundary
            evidence_entry = BoundaryEvidence(
                boundary_id=boundary.id,
                boundary_name=boundary.name,
                effect=boundary.rules.effect,
                decision=decision,
                similarities=similarities,
            )
            evidence.append(evidence_entry)

            logger.info(
                f"Boundary '{boundary.name}' ({boundary.type}): "
                f"{'ALLOW' if decision == 1 else 'BLOCK'} "
                f"(similarities: {[f'{s:.3f}' for s in similarities]})"
            )

        # Aggregate results with deny-first semantics (Issue #3 fix)
        # Phase 1: Check deny boundaries - if any match, immediate BLOCK
        # Phase 2: Check allow boundaries - all mandatory must pass for ALLOW

        # Separate boundaries by effect
        deny_results = [
            (b, dec, sims) for b, dec, sims in boundary_results
            if b.rules.effect == "deny"
        ]
        allow_results = [
            (b, dec, sims) for b, dec, sims in boundary_results
            if b.rules.effect == "allow"
        ]

        logger.debug(
            f"Aggregating results: {len(deny_results)} deny boundaries, "
            f"{len(allow_results)} allow boundaries"
        )

        # Phase 1: Check deny boundaries (short-circuit on match)
        for boundary, decision, similarities in deny_results:
            if decision == 1:  # Deny boundary matched (similarities ≥ thresholds)
                # Deny match → immediate BLOCK
                logger.info(
                    f"BLOCKED by deny boundary '{boundary.name}' (id: {boundary.id}). "
                    f"Similarities: {[f'{s:.3f}' for s in similarities]}"
                )
                result = ComparisonResult(
                    decision=0,  # BLOCK
                    slice_similarities=similarities,
                    boundaries_evaluated=len(applicable_boundaries),
                    timestamp=time.time(),
                    evidence=evidence,
                )
                logger.info(f"Final decision for intent {event.id}: BLOCK (deny match)")
                return result

        logger.debug("No deny boundaries matched. Checking allow boundaries...")

        # Phase 2: Check allow boundaries (all mandatory must pass)
        mandatory_allow_results = [
            (b, dec, sims) for b, dec, sims in allow_results
            if b.type == "mandatory"
        ]

        if not mandatory_allow_results:
            # No mandatory allow boundaries - default to BLOCK (fail-closed security)
            # (Policy decision: if no allow policies apply after deny check, block)
            logger.warning("No mandatory allow boundaries. Defaulting to BLOCK (fail-closed security).")
            final_decision = 0  # BLOCK
            final_similarities = [0.0, 0.0, 0.0, 0.0]
        else:
            # Check if ALL mandatory allow boundaries pass
            mandatory_decisions = [dec for _, dec, _ in mandatory_allow_results]
            all_mandatory_pass = all(dec == 1 for dec in mandatory_decisions)

            if all_mandatory_pass:
                final_decision = 1  # ALLOW
                # Use average similarities across all mandatory allow boundaries
                avg_similarities = [
                    sum(sims[i] for _, _, sims in mandatory_allow_results) / len(mandatory_allow_results)
                    for i in range(4)
                ]
                final_similarities = avg_similarities
                logger.info(
                    f"All {len(mandatory_allow_results)} mandatory allow boundaries passed. "
                    "Final decision: ALLOW"
                )
            else:
                final_decision = 0  # BLOCK
                # Use minimum similarities across mandatory allow boundaries
                min_similarities = [
                    min(sims[i] for _, _, sims in mandatory_allow_results)
                    for i in range(4)
                ]
                final_similarities = min_similarities
                # Find which boundary failed
                failed_boundaries = [
                    b.name for b, dec, _ in mandatory_allow_results if dec == 0
                ]
                logger.info(
                    f"BLOCKED: {len(failed_boundaries)} mandatory allow boundary(ies) failed: "
                    f"{', '.join(failed_boundaries)}"
                )

        result = ComparisonResult(
            decision=final_decision,
            slice_similarities=final_similarities,
            boundaries_evaluated=len(applicable_boundaries),
            timestamp=time.time(),
            evidence=evidence,
        )

        logger.info(
            f"Final decision for intent {event.id}: "
            f"{'ALLOW' if final_decision == 1 else 'BLOCK'} "
            f"(aggregated similarities: {[f'{s:.3f}' for s in final_similarities]})"
        )

        return result

    except Exception as e:
        logger.error(f"Error comparing intent {event.id}: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to compare intent: {str(e)}",
        )
