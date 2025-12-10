# Production Run Fix Plan v1.0

**Created**: 2025-11-13
**Status**: DRAFT
**Goal**: Fix all issues preventing correct policy enforcement in block mode

---

## Executive Summary

Production validation of Phase 2 (anchor-based encoding) revealed **6 critical issues** preventing correct policy enforcement:

1. **Enforcement mechanism broken** - Callbacks don't halt agent execution
2. **Missing applicability filtering** - All boundaries evaluated for every intent
3. **No allow/deny semantics** - Deny policies treated as allow policies
4. **Missing boundary evidence** - SDK shows "blocked by boundary unknown"
5. **Cold start timeouts** - Model loading exceeds SDK timeout
6. **Semantic bleed** - Cross-action similarities too high (read vs write ~0.86)

**Impact**: All 4 test scenarios failed. Delete and export operations allowed when they should block.

**Root Cause**: Pipeline missing Phase 2.1 applicability filtering from algo.md. Enforcement relies solely on callbacks instead of guards.

---

## Issue #1: Enforcement Mechanism Broken

### Observation
- SDK prints `AgentSecurityException` in callbacks
- Agent continues execution and returns "ALLOWED - Agent Response"
- Delete/export operations complete despite exceptions

### Inference
- Callback exceptions are caught and logged by LangGraph framework
- Callbacks cannot halt agent/tool execution (they're observability hooks, not guards)
- Need pre-execution guards that interrupt control flow

### Recommended Fix
**Priority**: CRITICAL | **Effort**: 2-3 hours

1. **SDK Changes** (`tupl-sdk-py/src/tupl_sdk/integrations/langchain.py`):
   - Add `TuplToolWrapper` class that wraps tools with pre-execution guard
   - Guard calls `compare_intent()` before tool execution
   - If BLOCK: raise `ToolException` (LangGraph-recognized error) instead of allowing execution
   - If ALLOW: proceed with tool call

2. **Keep Callbacks for Telemetry**:
   - `AgentCallback` remains for LLM call observation and telemetry
   - Enforcement happens in tool wrapper, not callback

3. **Update Demo**:
   - Wrap tools with `TuplToolWrapper.wrap_tool(tool, tupl_client, ...)`
   - Example:
     ```python
     from tupl_sdk.integrations.langchain import TuplToolWrapper

     wrapped_tools = [
         TuplToolWrapper.wrap_tool(tool, tupl_client, tenant_id, actor)
         for tool in tools
     ]
     agent = create_react_agent(model, wrapped_tools)
     ```

### Success Criteria
- ✅ DELETE operation raises `ToolException` and halts execution
- ✅ Agent returns error message instead of executing tool
- ✅ No "ALLOWED - Agent Response" for blocked operations

---

## Issue #2: Missing Applicability Filtering

### Observation
- READ scenarios end up BLOCK even when "Allow Read Operations" passes
- Management Plane logs show "Loaded 3 active boundaries for comparison" for every intent
- Non-applicable boundaries (e.g., "Allow Limited Writes" for READ) fail thresholds and contaminate min-mode decision

### Inference
- No candidate filtering (Phase 2.1 from algo.md "Inverted Index Lookup")
- All mandatory boundaries evaluated regardless of action/actor/resource match
- With min-mode aggregation, any non-applicable failure causes global BLOCK

### Recommended Fix
**Priority**: CRITICAL | **Effort**: 3-4 hours

1. **Add Applicability Filter** (`management-plane/app/endpoints/intents.py`):
   - Before FFI comparison, filter boundaries by deterministic pre-checks:
     ```python
     def is_boundary_applicable(intent: IntentEvent, boundary: DesignBoundary) -> bool:
         # Action match
         if intent.action not in boundary.constraints.action.actions:
             return False

         # Actor type match (if specified)
         if boundary.constraints.action.actor_types:
             if intent.actor.type not in boundary.constraints.action.actor_types:
                 return False

         # Resource type match (if specified)
         if boundary.constraints.resource.types:
             if intent.resource.type not in boundary.constraints.resource.types:
                 return False

         # Resource location match (if specified)
         if boundary.constraints.resource.locations:
             if intent.resource.location not in boundary.constraints.resource.locations:
                 return False

         # Data PII match (if specified)
         if boundary.constraints.data.pii is not None:
             if intent.data.pii != boundary.constraints.data.pii:
                 return False

         # Data volume match (if specified)
         if boundary.constraints.data.volume:
             if intent.data.volume != boundary.constraints.data.volume:
                 return False

         return True
     ```

2. **Update Comparison Endpoint**:
   - Filter: `applicable = [b for b in active if is_boundary_applicable(intent, b)]`
   - If no applicable boundaries: return default decision (ALLOW or configurable)
   - Only compare against applicable boundaries
   - Log: "Filtered to {N} applicable boundaries from {M} total"

3. **Add Tests** (`tests/test_applicability_filter.py`):
   - Test action mismatch (read intent vs write boundary)
   - Test actor type exclusion (agent intent vs user-only boundary)
   - Test resource type mismatch (api intent vs database boundary)
   - Test data attribute mismatch (pii=True vs pii=False)
   - Test multiple boundaries with mixed applicability

### Success Criteria
- ✅ READ intent only evaluates "Allow Read Operations" (1 boundary, not 3)
- ✅ WRITE by agent only evaluates "Allow Read Operations" (excludes "Allow Limited Writes" due to actor mismatch)
- ✅ DELETE by agent only evaluates "Block Risky Operations"
- ✅ Logs show "Filtered to N applicable boundaries"

---

## Issue #3: No Allow/Deny Semantics

### Observation
- "Block Risky Operations" is a deny policy by intention
- Logs show it "ALLOW"ing when DELETE matches (similarities ≥ thresholds)
- Final BLOCK only happens because other boundaries fail, not because deny matched

### Inference
- No `effect` field in policy rules
- All boundaries treated as allow policies in min-mode aggregation
- Deny boundaries should short-circuit to BLOCK when matched, regardless of other boundaries

### Recommended Fix
**Priority**: HIGH | **Effort**: 2-3 hours

1. **Extend Data Model** (`management-plane/app/types.py`):
   ```python
   class BoundaryRules(BaseModel):
       effect: Literal["allow", "deny"] = "allow"  # Add this field
       decision_mode: Literal["min", "weighted", "unanimous"]
       type: Literal["mandatory", "optional"]
       thresholds: SliceThresholds
   ```

2. **Update Aggregation Logic** (`management-plane/app/endpoints/intents.py`):
   ```python
   # Separate applicable boundaries by effect
   deny_boundaries = [b for b in applicable if b.rules.effect == "deny"]
   allow_boundaries = [b for b in applicable if b.rules.effect == "allow"]

   # Check deny boundaries first
   for boundary in deny_boundaries:
       result = compare_with_boundary(intent, boundary)
       if result.passes_all_thresholds():
           return ComparisonResponse(
               decision="BLOCK",
               matched_deny_boundary=boundary.id,
               evidence=[...]
           )

   # Then check allow boundaries (existing min-mode logic)
   mandatory_allow = [b for b in allow_boundaries if b.rules.type == "mandatory"]
   if not mandatory_allow:
       return default_decision  # No applicable allow policies

   # Existing min-mode: all mandatory allow must pass
   for boundary in mandatory_allow:
       result = compare_with_boundary(intent, boundary)
       if not result.passes_all_thresholds():
           return ComparisonResponse(
               decision="BLOCK",
               failed_allow_boundary=boundary.id,
               evidence=[...]
           )

   return ComparisonResponse(decision="ALLOW", evidence=[...])
   ```

3. **Update Seeded Policies** (`examples/langgraph_demo/policies.py`):
   ```python
   block_risky_ops = {
       "rules": {
           "effect": "deny",  # Add this
           "decision_mode": "min",
           "type": "mandatory",
           ...
       }
   }
   ```

4. **Add Tests**:
   - Test deny boundary match → immediate BLOCK
   - Test deny boundary non-match + allow boundary pass → ALLOW
   - Test deny + allow both match → BLOCK (deny wins)

### Success Criteria
- ✅ DELETE by agent → BLOCK from "Block Risky Operations" (deny match)
- ✅ READ by agent → ALLOW from "Allow Read Operations" (deny doesn't match, allow passes)
- ✅ Logs show "Blocked by deny boundary: block-risky-ops"

---

## Issue #4: Missing Boundary Evidence

### Observation
- SDK exceptions show "blocked by boundary unknown"
- `/intents/compare` response only returns `{decision, slice_similarities}`
- No boundary identifiers or per-boundary evidence

### Inference
- API contract doesn't include which policies matched/failed
- SDK can't populate meaningful error messages
- Debugging and auditing is impossible

### Recommended Fix
**Priority**: MEDIUM | **Effort**: 1-2 hours

1. **Extend Response Model** (`management-plane/app/types.py`):
   ```python
   class BoundaryEvidence(BaseModel):
       boundary_id: str
       boundary_name: str
       effect: Literal["allow", "deny"]
       slice_similarities: dict[str, float]
       thresholds: dict[str, float]
       passed: bool
       decision_impact: str  # "matched_deny", "failed_allow", "passed_allow"

   class ComparisonResponse(BaseModel):
       decision: Literal["ALLOW", "BLOCK"]
       evidence: list[BoundaryEvidence]
       matched_deny_boundary_id: str | None = None
       failed_allow_boundary_id: str | None = None
   ```

2. **Update Endpoint** (`management-plane/app/endpoints/intents.py`):
   - Build `evidence` list with all evaluated boundaries
   - Populate `matched_deny_boundary_id` or `failed_allow_boundary_id`

3. **Update SDK** (`tupl-sdk-py/src/tupl_sdk/client.py`):
   ```python
   if response.decision == "BLOCK":
       if response.matched_deny_boundary_id:
           boundary_id = response.matched_deny_boundary_id
       elif response.failed_allow_boundary_id:
           boundary_id = response.failed_allow_boundary_id
       else:
           boundary_id = "unknown"

       raise AgentSecurityException(
           f"Intent {intent.id} blocked by boundary {boundary_id}",
           evidence=response.evidence
       )
   ```

### Success Criteria
- ✅ Exception message shows "blocked by boundary block-risky-ops"
- ✅ Evidence includes all evaluated boundaries with scores
- ✅ Can trace which policy caused the decision

---

## Issue #5: Cold Start Timeouts

### Observation
- First request to Management Plane takes 10-20 seconds (model download + projection init)
- SDK default timeout is 5 seconds
- First intent comparison fails with timeout error

### Inference
- Sentence-transformers model loads lazily on first encode
- Projection matrices created on first use
- SDK doesn't account for cold start latency

### Recommended Fix
**Priority**: LOW | **Effort**: 30 min

1. **Add Warmup to Management Plane** (`management-plane/app/main.py`):
   ```python
   @app.on_event("startup")
   async def warmup():
       logger.info("Warming up encoder...")
       encoder = get_encoder()
       # Encode canonical strings to force model load + projection init
       encoder.encode("action is read")
       encoder.encode("resource_type is database")
       encoder.encode("sensitivity is internal")
       encoder.encode("authn is required")
       logger.info("Encoder warmed up")
   ```

2. **Increase SDK Initial Timeout** (`tupl-sdk-py/src/tupl_sdk/client.py`):
   ```python
   class TuplClient:
       def __init__(self, ..., initial_timeout: float = 30.0, timeout: float = 5.0):
           self._initial_timeout = initial_timeout
           self._timeout = timeout
           self._first_call = True

       def compare_intent(self, ...):
           timeout = self._initial_timeout if self._first_call else self._timeout
           response = requests.post(..., timeout=timeout)
           self._first_call = False
   ```

### Success Criteria
- ✅ First request succeeds within 30 seconds
- ✅ Subsequent requests use 5 second timeout
- ✅ No timeout errors in demo

---

## Issue #6: Semantic Bleed (Cross-Action Similarities)

### Observation
- READ vs WRITE similarity: ~0.86
- READ vs DELETE similarity: ~0.74
- Actor/risk slots often ≥0.80 across variants

### Inference
- Sentence-transformers embeddings are semantically smooth
- Adjacent verbs/roles are close in vector space
- Thresholds alone cannot express strict set membership
- This is expected behavior for semantic similarity

### Recommended Fix
**Priority**: LOW | **Effort**: Informational (no code changes)

**Strategy**: Rely on applicability filtering (Issue #2 fix) to handle strict membership.

1. **Applicability filter** handles exact action/actor/resource matching (deterministic)
2. **Anchor-based similarity** handles within-category nuance and synonyms
3. **Combined approach**:
   - Filter ensures only relevant boundaries are evaluated (read intents don't see write boundaries)
   - Anchors ensure "read customer data" matches "read user records" semantically
   - Cross-category bleed (read ↔ write) is prevented by applicability filter, not thresholds

**Optional Enhancement** (if needed in Week 4):
- Make anchor text more structured: `"action=read"` instead of `"action is read"`
- Reduces semantic bleed by using token-like format
- Only implement if applicability filter isn't sufficient

### Success Criteria
- ✅ Applicability filter prevents cross-action evaluation
- ✅ Within-category similarities remain high (0.90-0.95)
- ✅ Document expected behavior in algo.md

---

## Implementation Order

### Phase A: Critical Fixes (Issues #1, #2, #3)
**Estimated Time**: 8-10 hours
**Blocking**: Cannot validate system until these are fixed

1. **Issue #2: Applicability Filter** (3-4 hours)
   - Implement `is_boundary_applicable()` function
   - Update comparison endpoint to filter boundaries
   - Add unit tests for filter logic
   - **Why first**: Unblocks testing by reducing false BLOCKs

2. **Issue #3: Allow/Deny Semantics** (2-3 hours)
   - Add `effect` field to data model
   - Update aggregation logic (deny-first, then allow)
   - Update seeded policies with `effect: "deny"`
   - Add unit tests for deny semantics
   - **Why second**: Builds on applicability filter

3. **Issue #1: Enforcement Guards** (2-3 hours)
   - Implement `TuplToolWrapper` in SDK
   - Update demo to wrap tools
   - Test that DELETE raises `ToolException`
   - **Why third**: Validates that fixes #2 and #3 actually work

### Phase B: Evidence & Observability (Issue #4)
**Estimated Time**: 1-2 hours
**Non-blocking**: Can validate basic enforcement without this

4. **Issue #4: Boundary Evidence** (1-2 hours)
   - Extend response model with evidence
   - Update endpoint to build evidence list
   - Update SDK to use boundary IDs in exceptions

### Phase C: Polish (Issues #5, #6)
**Estimated Time**: 30 min
**Nice-to-have**: Improves UX but not blocking

5. **Issue #5: Cold Start** (30 min)
   - Add warmup to Management Plane startup
   - Increase SDK initial timeout

6. **Issue #6: Semantic Bleed** (0 min - documentation only)
   - Document expected behavior
   - Note that applicability filter prevents cross-action bleed

---

## Validation Plan

### After Phase A (Critical Fixes)

**Test Scenario 1: READ Operation**
- Intent: `action=read, actor=agent, resource=api`
- Expected:
  - ✅ Applicability filter selects only "Allow Read Operations" (1 boundary)
  - ✅ Similarity scores ≥ thresholds
  - ✅ Decision: ALLOW
  - ✅ Agent completes search operation

**Test Scenario 2: WRITE by Agent**
- Intent: `action=write, actor=agent, resource=database`
- Expected:
  - ✅ Applicability filter excludes "Allow Limited Writes" (actor mismatch)
  - ✅ Only "Allow Read Operations" evaluated (read ≠ write, fails action)
  - ✅ Decision: BLOCK
  - ✅ Tool wrapper raises `ToolException`, agent halts

**Test Scenario 3: DELETE by Agent**
- Intent: `action=delete, actor=agent, resource=database`
- Expected:
  - ✅ Applicability filter selects "Block Risky Operations" (deny policy)
  - ✅ Similarity scores ≥ thresholds (deny match)
  - ✅ Decision: BLOCK (deny short-circuits)
  - ✅ Tool wrapper raises `ToolException`, agent halts
  - ✅ (After Phase B) Exception shows "blocked by boundary block-risky-ops"

**Test Scenario 4: EXPORT by Agent**
- Intent: `action=export, actor=agent, resource=database, data.volume=bulk`
- Expected:
  - ✅ Applicability filter selects "Block Risky Operations"
  - ✅ Deny match → BLOCK
  - ✅ Agent halted

### Success Metrics
- ✅ 0/4 scenarios currently pass → 4/4 scenarios pass after fixes
- ✅ No "ALLOWED - Agent Response" for DELETE/EXPORT
- ✅ Correct boundaries evaluated (1-2 instead of all 3)
- ✅ Logs show applicability filtering and deny-first aggregation

---

## Files to Modify

### Phase A: Critical Fixes

1. **`management-plane/app/endpoints/intents.py`** (Issues #2, #3)
   - Add `is_boundary_applicable()` function (~40 lines)
   - Update `/intents/compare` endpoint (~60 lines modified)
   - Add deny-first aggregation logic (~30 lines)

2. **`management-plane/app/types.py`** (Issue #3)
   - Add `effect` field to `BoundaryRules` (1 line)

3. **`examples/langgraph_demo/policies.py`** (Issue #3)
   - Add `effect: "deny"` to Block Risky Operations (1 line)

4. **`tupl-sdk-py/src/tupl_sdk/integrations/langchain.py`** (Issue #1)
   - Add `TuplToolWrapper` class (~80 lines)

5. **`examples/langgraph_demo/demo_prebuilt.py`** (Issue #1)
   - Wrap tools with `TuplToolWrapper` (~10 lines modified)

6. **`management-plane/tests/test_applicability_filter.py`** (NEW - Issue #2)
   - Unit tests for applicability logic (~150 lines)

7. **`management-plane/tests/test_deny_semantics.py`** (NEW - Issue #3)
   - Unit tests for deny-first aggregation (~100 lines)

### Phase B: Evidence

8. **`management-plane/app/types.py`** (Issue #4)
   - Add `BoundaryEvidence` model (~15 lines)
   - Extend `ComparisonResponse` (~5 lines)

9. **`management-plane/app/endpoints/intents.py`** (Issue #4)
   - Build evidence list (~20 lines modified)

10. **`tupl-sdk-py/src/tupl_sdk/client.py`** (Issue #4)
    - Use boundary IDs in exceptions (~10 lines modified)

### Phase C: Polish

11. **`management-plane/app/main.py`** (Issue #5)
    - Add warmup handler (~10 lines)

12. **`tupl-sdk-py/src/tupl_sdk/client.py`** (Issue #5)
    - Add initial timeout parameter (~5 lines)

---

## Risk Assessment

### High Risk
- **Applicability filter correctness**: If filter is too strict, valid operations will be blocked.
  - Mitigation: Comprehensive unit tests; log all filtering decisions

- **Deny semantics breaking existing flows**: Changing aggregation logic could affect Week 3 telemetry storage.
  - Mitigation: Maintain backward compatibility; add `effect` as optional field defaulting to "allow"

### Medium Risk
- **Tool wrapper integration with LangGraph**: Framework may not propagate `ToolException` as expected.
  - Mitigation: Test with multiple LangGraph patterns (prebuilt, StateGraph); document workarounds

### Low Risk
- **Cold start timeout**: May need tuning for different environments.
  - Mitigation: Make timeout configurable; document in QUICKSTART.md

---

## Success Criteria (Overall)

### Functional Requirements
- ✅ READ operations by agent → ALLOW
- ✅ WRITE operations by agent → BLOCK (not in any allow policy after filtering)
- ✅ DELETE operations by agent → BLOCK (deny policy match)
- ✅ EXPORT operations by agent → BLOCK (deny policy match)

### Technical Requirements
- ✅ Applicability filter reduces evaluated boundaries by 50-67% on average
- ✅ Deny boundaries short-circuit to BLOCK when matched
- ✅ SDK exceptions include boundary IDs
- ✅ No timeout errors on first request
- ✅ All existing tests continue to pass

### Performance Requirements
- ✅ Applicability filter adds <1ms per intent (deterministic checks)
- ✅ Overall latency remains <100ms P50 (10ms target)

---

## Next Session Handoff

**After completing this plan**:

1. Update STATUS.md with Phase A/B/C completion status
2. Document actual similarity improvements in production
3. Create Week 3 plan for database persistence
4. Consider adding:
   - Inverted index implementation (algo.md Phase 2.1 optimization)
   - Boundary caching (anchor generation is expensive)
   - Telemetry for applicability filter (how many boundaries filtered per intent)

**Estimated Total Time**: 10-12 hours across Phase A, B, C

**Critical Path**: Phase A (Issues #1, #2, #3) must complete before system is usable.
