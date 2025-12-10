# Deny Short-Circuit Optimization

**Created**: 2025-11-14
**Status**: IDENTIFIED - Not Implemented
**Priority**: MEDIUM (Performance optimization, not correctness issue)

---

## Problem

Current implementation evaluates **all applicable boundaries** before checking deny semantics, even though a single deny match should immediately BLOCK.

### Current Flow
```
1. Filter to applicable boundaries (N boundaries)
2. Call sandbox.compare() for ALL N boundaries
3. Separate results by effect (deny vs allow)
4. Check deny results - if any match, BLOCK
5. Otherwise check allow results
```

### Inefficiency
If the first boundary is a deny that matches, we still call `sandbox.compare()` for all remaining N-1 boundaries unnecessarily.

---

## Impact

**Performance**: Wasted FFI calls and semantic comparisons
- Each `sandbox.compare()` call involves:
  - Anchor generation (4 slots × up to 16 anchors)
  - Encoding to 32-d vectors
  - FFI boundary crossing (Python → Rust)
  - Cosine similarity calculations (4 slots × M anchors)

**Example Scenario**:
- 10 applicable boundaries: 3 deny, 7 allow
- Deny boundary #1 matches → should BLOCK immediately
- Current: Makes 10 FFI calls (100% waste after first match)
- Optimal: Makes 1 FFI call (90% savings)

---

## Proposed Solution

Evaluate deny boundaries **before** allow boundaries, with early return:

### Optimized Flow
```python
# After applicability filtering...
applicable_boundaries = [...]  # Filtered list

# Separate by effect BEFORE comparison
deny_boundaries = [b for b in applicable if b.rules.effect == "deny"]
allow_boundaries = [b for b in applicable if b.rules.effect == "allow"]

# Phase 1: Check deny boundaries (short-circuit on first match)
for deny_boundary in deny_boundaries:
    decision, similarities = sandbox.compare(intent, deny_boundary)
    if decision == 1:  # Deny match
        return ComparisonResult(decision=0, slice_similarities=similarities)

# Phase 2: Check allow boundaries (all must pass)
allow_results = []
for allow_boundary in allow_boundaries:
    decision, similarities = sandbox.compare(intent, allow_boundary)
    allow_results.append((allow_boundary, decision, similarities))

# Aggregate allow results...
```

---

## Example Cases

### Case 1: Deny Match (First Boundary)
```
Applicable: [deny1, deny2, allow1, allow2, allow3]

Current:  5 FFI calls
Optimal:  1 FFI call (deny1 matches → immediate return)
Savings:  80%
```

### Case 2: Deny Match (Third Boundary)
```
Applicable: [deny1, deny2, deny3, allow1, allow2]

Current:  5 FFI calls
Optimal:  3 FFI calls (deny1, deny2 don't match; deny3 matches → return)
Savings:  40%
```

### Case 3: No Deny Match (Allow Check Required)
```
Applicable: [deny1, deny2, allow1, allow2]

Current:  4 FFI calls
Optimal:  4 FFI calls (must check all: 2 deny + 2 allow)
Savings:  0% (no savings, but same behavior)
```

### Case 4: No Deny Boundaries
```
Applicable: [allow1, allow2, allow3]

Current:  3 FFI calls
Optimal:  3 FFI calls (skip deny phase, check all allow)
Savings:  0% (no savings, but same behavior)
```

---

## Implementation Effort

**Estimated**: 1-2 hours

**Changes Required**:
- `management-plane/app/endpoints/intents.py` (compare_intent function)
  - Move effect-based separation before the comparison loop
  - Add early return in deny loop
  - Simplify allow loop (no longer needs to collect all results upfront)

**Testing**:
- Update `tests/test_deny_semantics.py::test_deny_wins_over_allow_when_both_match`
  - Change expected call count from 2 to 1

---

## Trade-offs

### Pros
- **Performance**: Up to 90% reduction in FFI calls for deny-heavy scenarios
- **Latency**: Faster BLOCK decisions (sub-millisecond improvement per saved call)
- **Clearer semantics**: Code mirrors the deny-first policy intent

### Cons
- **Minimal**: No functional change, pure optimization
- **Logging**: Lose per-boundary logging for unevaluated boundaries (acceptable trade-off)

---

## Recommendation

**Defer to Week 4 (Hardening & Performance)**

Rationale:
- Current implementation is **functionally correct** (deny wins, all tests pass except short-circuit assertion)
- Performance impact is low for MVP scenarios (typically 1-3 boundaries)
- Week 3-4 priorities are higher (database persistence, production readiness)
- Can optimize after profiling shows this as a bottleneck

---

## Test Update Required

When implementing, update this test:

```python
# tests/test_deny_semantics.py
async def test_deny_wins_over_allow_when_both_match(...):
    # ...
    assert result.decision == 0, "Deny should win when both match"

    # BEFORE optimization:
    assert mock_sandbox_instance.compare.call_count == 2  # Both boundaries evaluated

    # AFTER optimization:
    assert mock_sandbox_instance.compare.call_count == 1  # Deny only, short-circuit
```

---

## References

- Original issue: Session 17, Task 7 (deny semantics unit tests)
- Related code: `management-plane/app/endpoints/intents.py:277-382`
- Test file: `management-plane/tests/test_deny_semantics.py:180-214`
