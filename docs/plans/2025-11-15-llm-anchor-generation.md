# Implementation Plan: LLM-Based Anchor Generation with Pre-Encoding at Installation

**Date**: 2025-11-15
**Author**: Claude Code
**Status**: Proposed

## Overview
Replace template-based anchor generation with LLM-generated anchors using **Google GenAI SDK (`google-genai`)** with **Gemini 2.5 Flash Lite**. Move encoding from enforcement-time (lazy) to installation-time (eager) to eliminate N HTTP round-trips during enforcement.

**Key Innovation**: Single unified `build_rule_anchors()` function that works for ALL rule families - no per-family methods needed.

## Key Decisions
- ✅ Use `google-genai` (unified SDK)
- ✅ Gemini 2.5 Flash Lite model (fast, cheap, perfect for anchor generation)
- ✅ Structured outputs via Pydantic `response_schema`
- ✅ LLM only (replace templates entirely)
- ✅ Fail-fast on LLM errors (strict mode)
- ✅ Content hash-based caching
- ✅ Configuration file for API key (read from demo .env)
- ✅ **UNIFIED FUNCTION**: Single `build_rule_anchors(rule, family_id)` for all families

## Architecture

### Current Flow (Suboptimal)
```
SDK → Data Plane gRPC → Bridge (store dict)
                            ↓ [LATER: Enforcement]
                    HTTP /encode/rule/{family} (PER RULE!) ⚠️
                            ↓
                    Management Plane (templates)
                            ↓
                    Return anchors
```

### Proposed Flow (Optimized)
```
SDK → Data Plane gRPC → HTTP /encode/rule/{family} (ONE-TIME)
                            ↓
                    Management Plane
                            ↓ [LLM CALL]
                    Gemini 2.5 Flash Lite
                            ↓
                    Generate semantic anchors
                            ↓
                    Encode to 32-dim vectors
                            ↓ Store pre-encoded
                    Bridge (rule + anchors)

[ENFORCEMENT]
SDK → Data Plane → HTTP /encode/intent (1 call)
                → Bridge.get_rule_anchors() (memory)
                → FFI compare
                → Decision
```

## Implementation Steps

### Step 1: Add Google GenAI Dependency
**File**: `management-plane/pyproject.toml`

Add dependency:
```toml
dependencies = [
    # ... existing
    "google-genai>=1.0.0",
]
```

### Step 2: Create LLM Anchor Generator
**File**: `management-plane/app/llm_anchor_generator.py` (NEW)

Single class with one method for all rule families:

```python
from google import genai
from google.genai import types
from pydantic import BaseModel
import hashlib
import json
from functools import lru_cache

class AnchorSlots(BaseModel):
    """Structured output schema for LLM-generated anchors."""
    action: list[str]
    resource: list[str]
    data: list[str]
    risk: list[str]

class LLMAnchorGenerator:
    """
    Unified LLM-based anchor generator for all rule families.

    Uses Gemini 2.5 Flash Lite with structured outputs to generate
    semantic anchor descriptions from rule definitions.
    """

    def __init__(self, api_key: str, model: str = "gemini-2.5-flash-lite"):
        self.client = genai.Client(api_key=api_key)
        self.model = model
        self._cache = {}  # Content hash → AnchorSlots

    def _compute_cache_key(self, rule: dict, family_id: str) -> str:
        """Compute content hash for caching."""
        content = json.dumps({"rule": rule, "family": family_id}, sort_keys=True)
        return hashlib.sha256(content.encode()).hexdigest()

    async def generate_rule_anchors(
        self,
        rule: dict,
        family_id: str
    ) -> AnchorSlots:
        """
        Generate semantic anchors for ANY rule family.

        Args:
            rule: Rule dict with family-specific fields
            family_id: Rule family identifier (e.g., "tool_whitelist")

        Returns:
            AnchorSlots with 2-4 semantic descriptions per slot

        Raises:
            ValueError: If LLM call fails (fail-fast)
        """
        # Check cache
        cache_key = self._compute_cache_key(rule, family_id)
        if cache_key in self._cache:
            return self._cache[cache_key]

        # Build prompt
        prompt = f"""You are a semantic security expert analyzing LLM application security rules.

Generate natural language anchor descriptions for this {family_id} rule that will be used for semantic similarity comparison against runtime tool calls.

Rule Definition:
```json
{json.dumps(rule, indent=2)}
```

For each of the 4 semantic slots (action, resource, data, risk), generate 2-4 concise natural language descriptions that capture what this rule allows or constrains:

- **Action slot**: What operations/methods are allowed or required
- **Resource slot**: What tools, APIs, or resources this rule applies to
- **Data slot**: Data characteristics, sensitivity, or constraints
- **Risk slot**: Security requirements, validation strictness, or authentication needs

Make descriptions:
- Specific to this rule's constraints
- Natural language (not template syntax)
- Semantically rich for embedding comparison
- 5-15 words each

Return valid JSON matching the required schema."""

        try:
            # Call LLM with structured output
            response = self.client.models.generate_content(
                model=self.model,
                contents=prompt,
                config=types.GenerateContentConfig(
                    response_mime_type='application/json',
                    response_schema=AnchorSlots,
                    temperature=0.3,  # Low temperature for consistency
                ),
            )

            # Parse response
            anchors = AnchorSlots.model_validate_json(response.text)

            # Cache result
            self._cache[cache_key] = anchors

            return anchors

        except Exception as e:
            # Fail-fast: LLM errors block rule installation
            raise ValueError(
                f"Failed to generate anchors for {family_id} rule: {str(e)}"
            ) from e

# Singleton instance
_generator = None

def get_llm_generator() -> LLMAnchorGenerator:
    """Get or create singleton LLM anchor generator."""
    global _generator
    if _generator is None:
        from app.config import GOOGLE_API_KEY, GEMINI_MODEL
        _generator = LLMAnchorGenerator(
            api_key=GOOGLE_API_KEY,
            model=GEMINI_MODEL
        )
    return _generator
```

### Step 3: Update Rule Encoding - UNIFIED FUNCTION
**File**: `management-plane/app/rule_encoding.py` (MAJOR REFACTOR)

**Remove all template-based functions** (lines 86-327).

**Replace with single unified function**:

```python
"""
Rule-to-Anchor Conversion for Layer-Based Enforcement (v1.3).

NOW USES LLM-BASED GENERATION (no templates).
Single unified function for all rule families.
"""

import logging
from typing import Any

import numpy as np

from app.encoding import get_encoder_model, get_projection_matrix
from app.llm_anchor_generator import get_llm_generator

logger = logging.getLogger(__name__)


# ============================================================================
# Helper Functions (unchanged)
# ============================================================================

def encode_anchor_text(text: str, slot_name: str, seed: int) -> np.ndarray:
    """Encode a single anchor text to a 32-dim vector."""
    # ... existing implementation unchanged ...


def encode_anchor_list(
    anchors: list[str],
    slot_name: str,
    seed: int
) -> tuple[list[list[float]], int]:
    """Encode a list of anchor strings to a padded array."""
    # ... existing implementation unchanged ...


# ============================================================================
# UNIFIED Rule-to-Anchor Conversion (ALL FAMILIES)
# ============================================================================

async def build_rule_anchors(rule: dict[str, Any], family_id: str) -> dict[str, Any]:
    """
    Build anchor arrays for ANY rule family using LLM generation.

    This single function replaces all family-specific template functions.
    Works for ToolWhitelist, ToolParamConstraint, and any future rule families.

    Args:
        rule: Rule dict with family-specific fields
        family_id: Rule family identifier (e.g., "tool_whitelist")

    Returns:
        RuleAnchors dict with embedded anchor arrays

    Raises:
        ValueError: If LLM generation fails (fail-fast)
    """
    logger.info(f"Generating anchors for {family_id} rule: {rule.get('rule_id', 'unknown')}")

    # Generate anchor texts via LLM
    llm_generator = get_llm_generator()
    anchors = await llm_generator.generate_rule_anchors(rule, family_id)

    logger.debug(
        f"LLM generated anchors: "
        f"{len(anchors.action)} action, {len(anchors.resource)} resource, "
        f"{len(anchors.data)} data, {len(anchors.risk)} risk"
    )

    # Encode anchors to 32-dim vectors
    action_anchors, action_count = encode_anchor_list(anchors.action, "action", 42)
    resource_anchors, resource_count = encode_anchor_list(anchors.resource, "resource", 43)
    data_anchors, data_count = encode_anchor_list(anchors.data, "data", 44)
    risk_anchors, risk_count = encode_anchor_list(anchors.risk, "risk", 45)

    logger.info(
        f"Encoded {family_id} rule: "
        f"{action_count} action, {resource_count} resource, "
        f"{data_count} data, {risk_count} risk anchors"
    )

    return {
        "action_anchors": action_anchors,
        "action_count": action_count,
        "resource_anchors": resource_anchors,
        "resource_count": resource_count,
        "data_anchors": data_anchors,
        "data_count": data_count,
        "risk_anchors": risk_anchors,
        "risk_count": risk_count,
    }


# ============================================================================
# Backward Compatibility Aliases (optional - can remove later)
# ============================================================================

async def build_tool_whitelist_anchors(rule: dict[str, Any]) -> dict[str, Any]:
    """Build anchors for ToolWhitelist rule (calls unified function)."""
    return await build_rule_anchors(rule, "tool_whitelist")


async def build_tool_param_constraint_anchors(rule: dict[str, Any]) -> dict[str, Any]:
    """Build anchors for ToolParamConstraint rule (calls unified function)."""
    return await build_rule_anchors(rule, "tool_param_constraint")
```

**Key Benefits**:
- ✅ Zero tech debt: Adding new rule families requires NO new code
- ✅ Single source of truth for anchor generation
- ✅ LLM prompt adapts to any rule structure automatically
- ✅ Easy to maintain and test

### Step 4: Configuration Management
**File**: `management-plane/app/config.py` (NEW)

```python
"""Configuration for Management Plane."""

import os
from pathlib import Path
from dotenv import load_dotenv

# Load from demo .env
demo_env_path = Path(__file__).parent.parent.parent / "examples/langgraph_demo/.env"
if demo_env_path.exists():
    load_dotenv(demo_env_path)

GOOGLE_API_KEY = os.getenv("GOOGLE_API_KEY")
if not GOOGLE_API_KEY:
    raise ValueError(
        "GOOGLE_API_KEY not found. "
        "Please set it in examples/langgraph_demo/.env"
    )

GEMINI_MODEL = "gemini-2.5-flash-lite"  # Fast, cheap, perfect for anchors
```

### Step 5: Update Encoding Endpoints to Async
**File**: `management-plane/app/endpoints/encoding.py` (MODIFY)

Update endpoints to call unified function:

```python
@router.post("/rule/tool_whitelist", response_model=RuleAnchors)
async def encode_tool_whitelist_rule(rule: dict) -> RuleAnchors:
    """Encode a ToolWhitelist rule to anchor arrays."""
    try:
        from app.rule_encoding import build_rule_anchors

        # Call unified function
        anchors = await build_rule_anchors(rule, "tool_whitelist")

        logger.debug(f"ToolWhitelist rule encoded successfully")
        return anchors

    except ValueError as e:
        # LLM generation error
        logger.error(f"Failed to generate anchors: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Anchor generation failed: {str(e)}"
        )
    except Exception as e:
        logger.error(f"Failed to encode rule: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Encoding failed: {str(e)}"
        )


@router.post("/rule/tool_param_constraint", response_model=RuleAnchors)
async def encode_tool_param_constraint_rule(rule: dict) -> RuleAnchors:
    """Encode a ToolParamConstraint rule to anchor arrays."""
    try:
        from app.rule_encoding import build_rule_anchors

        # Call unified function
        anchors = await build_rule_anchors(rule, "tool_param_constraint")

        logger.debug(f"ToolParamConstraint rule encoded successfully")
        return anchors

    except ValueError as e:
        logger.error(f"Failed to generate anchors: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Anchor generation failed: {str(e)}"
        )
    except Exception as e:
        logger.error(f"Failed to encode rule: {e}", exc_info=True)
        raise HTTPException(
            status_code=500,
            detail=f"Encoding failed: {str(e)}"
        )


# For future rule families, just add:
# @router.post("/rule/{family_id}", response_model=RuleAnchors)
# async def encode_generic_rule(family_id: str, rule: dict) -> RuleAnchors:
#     return await build_rule_anchors(rule, family_id)
```

### Step 6: Data Plane Pre-Encoding
**File**: `tupl_data_plane/tupl_dp/bridge/src/grpc_server.rs` (MODIFY)

Update `install_rules()` to encode during installation:

```rust
async fn install_rules(&self, request: Request<InstallRulesRequest>)
    -> Result<Response<InstallRulesResponse>, Status> {

    let req = request.into_inner();

    for proto_rule in req.rules {
        // 1. Convert proto to Bridge rule
        let bridge_rule = convert_proto_to_bridge(proto_rule)?;

        // 2. ENCODE rule to anchors (HTTP call to Management Plane)
        let anchors = self.encode_rule_during_installation(&bridge_rule).await
            .map_err(|e| Status::internal(format!("Failed to encode rule: {}", e)))?;

        // 3. Store ENCODED rule with anchors
        self.bridge.write().add_rule_with_anchors(bridge_rule, anchors)?;
    }

    // Return success response
}

async fn encode_rule_during_installation(&self, rule: &Arc<dyn RuleInstance>)
    -> Result<RuleVector, String> {
    // Same implementation as current encode_rule() in enforcement_engine.rs
    // But called during installation, not enforcement
}
```

**File**: `tupl_data_plane/tupl_dp/bridge/src/bridge.rs` (MODIFY)

Add anchor storage:

```rust
pub struct Bridge {
    tables: HashMap<(LayerId, FamilyId), Arc<RwLock<RuleTable>>>,
    rule_anchors: Arc<RwLock<HashMap<String, RuleVector>>>,  // NEW
}

impl Bridge {
    pub fn add_rule_with_anchors(
        &mut self,
        rule: Arc<dyn RuleInstance>,
        anchors: RuleVector
    ) -> Result<(), String> {
        let table = self.get_or_create_table(rule.layer(), rule.family_id());
        table.write().add_rule(rule.clone());

        self.rule_anchors.write().insert(
            rule.rule_id().to_string(),
            anchors
        );

        Ok(())
    }

    pub fn get_rule_anchors(&self, rule_id: &str) -> Option<RuleVector> {
        self.rule_anchors.read().get(rule_id).cloned()
    }
}
```

**File**: `tupl_data_plane/tupl_dp/bridge/src/enforcement_engine.rs` (MODIFY)

Remove lazy encoding:

```rust
impl EnforcementEngine {
    pub async fn enforce(&self, intent_json: &str) -> Result<EnforcementResult, String> {
        // 1. Encode intent (unchanged)
        let intent_vector = self.encode_intent(intent_json).await?;

        // 2. Get rules (unchanged)
        let rules = self.get_rules_for_layer(layer)?;

        // 3. Evaluate each rule
        for rule in rules {
            // GET PRE-ENCODED ANCHORS (memory lookup - NO HTTP!)
            let rule_vector = self.bridge
                .get_rule_anchors(rule.rule_id())
                .ok_or_else(|| {
                    format!(
                        "Rule '{}' has no pre-encoded anchors. Was it installed correctly?",
                        rule.rule_id()
                    )
                })?;

            // 4. Compare (unchanged)
            let result = self.compare_with_sandbox(&intent_vector, &rule_vector, &rule)?;

            if result.decision == 0 {
                return Ok(EnforcementResult { decision: 0, ... });
            }
        }

        Ok(EnforcementResult { decision: 1, ... })
    }

    // REMOVE ENTIRELY:
    // - encode_rule()
    // - get_or_encode_rule()
    // - EmbeddingCache struct
    // - All cache-related code
}
```

## Summary of Changes

### Zero Tech Debt Architecture
**Before**: Separate function per rule family
```python
build_tool_whitelist_anchors()
build_tool_param_constraint_anchors()
build_network_egress_anchors()  # Need to add
build_prompt_assembly_anchors()  # Need to add
# ... 14 families = 14 functions!
```

**After**: Single unified function
```python
build_rule_anchors(rule, family_id)
# Works for ALL current and future families!
```

### Files Modified/Created

**Management Plane (Python)**:
1. `pyproject.toml` - Add `google-genai>=1.0.0`
2. `app/config.py` (NEW) - Load API key
3. `app/llm_anchor_generator.py` (NEW) - LLM client + single method
4. `app/rule_encoding.py` (REFACTOR) - Delete 240 lines, replace with single function
5. `app/endpoints/encoding.py` (MODIFY) - Make async, call unified function

**Data Plane (Rust)**:
1. `src/grpc_server.rs` (MODIFY) - Encode during installation
2. `src/bridge.rs` (MODIFY) - Add anchor storage
3. `src/enforcement_engine.rs` (SIMPLIFY) - Remove 150+ lines of lazy encoding

### Performance

| Metric | Before | After |
|--------|--------|-------|
| Installation (per rule) | ~50ms | ~2000ms ⚠️ (LLM call) |
| Enforcement (1 rule) | ~50ms | ~10ms ✅ |
| Enforcement (10 rules) | ~500ms | ~15ms ✅ |
| Enforcement (100 rules) | ~5000ms | ~50ms ✅ |

**Key Insight**: Installation happens offline (admin action), so 2s latency is acceptable. Enforcement happens in hot path (every tool call), so 10x improvement is critical.

### Testing Plan
1. Add `google-genai` dependency
2. Implement `config.py` and `llm_anchor_generator.py`
3. Refactor `rule_encoding.py` to single function
4. Test with `demo_diagnostic.py`
5. Compare LLM vs template anchor similarity scores
6. Implement Rust pre-encoding changes
7. Full E2E test with rule installation

## Benefits

### Developer Experience
- ✅ **Zero maintenance overhead**: New rule families require NO code changes
- ✅ **Single source of truth**: All anchor generation in one function
- ✅ **Better semantic quality**: LLM generates contextual, natural language descriptions
- ✅ **Easy to debug**: LLM prompts are visible and tunable

### Performance
- ✅ **10x faster enforcement**: 10ms vs 50-200ms (no HTTP calls in hot path)
- ✅ **Predictable latency**: No cache misses, no network variability
- ✅ **Scales linearly**: 100 rules = same ~50ms enforcement time

### Architecture
- ✅ **Clean separation**: Offline (slow) vs online (fast) operations
- ✅ **No tech debt**: Future-proof for all 14 rule families
- ✅ **Cacheable**: Content hash ensures identical rules reuse anchors

## Risks & Mitigations

**Risk**: LLM API failures block rule installation
**Mitigation**: Strict fail-fast with clear error message, retry at application level

**Risk**: LLM generates poor quality anchors
**Mitigation**: Test with diagnostic demo, compare similarity scores, iterate on prompts

**Risk**: Installation latency increases
**Mitigation**: Acceptable for offline operation, cache by content hash reduces redundant calls

**Risk**: API costs increase
**Mitigation**: Gemini 2.5 Flash Lite is extremely cheap (~$0.001 per rule), cache eliminates redundant calls

## Next Steps

1. ✅ Get approval for plan
2. Implement Phase 1: Management Plane changes (Python)
   - Add dependency
   - Create LLM generator
   - Refactor rule_encoding.py
3. Test Phase 1 with diagnostic demo
4. Implement Phase 2: Data Plane changes (Rust)
   - Pre-encoding at installation
   - Remove lazy encoding
5. Full E2E validation
6. Performance benchmarking
7. Production deployment
