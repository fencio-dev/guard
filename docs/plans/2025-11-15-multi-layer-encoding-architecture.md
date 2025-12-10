# Multi-Layer Semantic Encoding Architecture (Future Optimization)

**Date**: 2025-11-15  
**Status**: Draft design – not yet implemented  
**Scope**: Cross-layer semantic encoding + comparison, to be applied once all layered rules are integrated (beyond L4).

This document captures a **forward-looking design** for unifying semantic encoding and comparison across all rule families (L1–L4, etc.) without incurring per-layer encoder tech debt. It is **not** the minimal L4 fix; that will be implemented separately as a constrained change that still fits within this architecture.

---

## Goals

- **Single semantic space** shared by all layers (no per-family embedding models or bespoke vector layouts).
- **Configurable per-family views** over that space (which slices, which thresholds, how to aggregate).
- **Anchors and rules as data**, not code (templates, configs) so new rule families can be added mostly by configuration.
- Preserve **determinism** and existing seeds/projection logic (128-dim, 4×32 slots).
- Avoid future tech debt from ad hoc “Lx-only” encoders.

---

## 1. One Canonical Semantic Space, Many Views

We keep the existing 128-dim layout as the **canonical semantic space**:

- 4 slices × 32 dims:
  - **Action slice** – what is being done.
  - **Resource slice** – who/what is touched (tool, API, table, file, principal).
  - **Data slice** – what information flows (PII, secrets, schema elements).
  - **Risk slice** – context/risk markers (caller identity, environment, channel, trust).

We treat these slices as **semantic facets**:

- **Action facet** → action slice  
- **Target/Resource facet** → resource slice  
- **Content/Data facet** → data slice  
- **Context/Risk facet** → risk slice  

Every rule family (L1–L4, etc.) expresses **how it views** this space by:

- Selecting a subset of facets/slices it cares about.
- Providing thresholds per slice or facet.
- Choosing an aggregation mode (min, max, mean, weighted mean, etc.).

Crucially, there is **only one semantic space** and one encoder; rule families do not get bespoke embedding models or layouts.

---

## 2. Generic Encoding Pipeline, Configured Per Family

The encoding pipeline is **generic**. Per-family behavior is expressed via **configuration/metadata**, not custom encoder branches.

### 2.1 IntentEvent → Facet Views

We introduce a generic function:

```python
def build_facets(intent: IntentEvent) -> FacetTextBundle:
    ...
```

Where `FacetTextBundle` is a structured object with text (or tokenizable) views for each facet, for example:

- `action_text`
  - e.g., “User attempts to CALL tool `db.query` to read customer emails”
- `target_text`
  - e.g., “Tool: db.query, table: customers, columns: email”
- `content_text`
  - e.g., “Potential PII: email, customer identifiers”
- `context_text`
  - e.g., “Agent: support-assistant, environment: prod, tenant: X”

The function:

- **Knows nothing about layers or rule families.**
- Performs “best effort semantic serialization” of the event into 3–4 textual channels (facets) using existing IntentEvent fields (including tool-related fields for L4).

### 2.2 Facet Text → 128-dim Vector

We reuse the existing embedding model and projection:

- Each facet is encoded into a base embedding.
- Each facet is projected into its 32-dim slot:
  - Action facet → action slice.
  - Target facet → resource slice.
  - Content facet → data slice.
  - Context facet → risk slice.
- The final 128-dim vector is the **same** for all rule families.

The encoder itself:

- Has **no awareness** of layers/families.
- Continues to respect determinism and fixed seeds.

### 2.3 Rule Family Config (No Custom Encoder)

Each rule family defines a **config** that declares how it uses the shared space. Example (conceptual YAML):

```yaml
family: L4_TOOL_GATEWAY
facets:
  use: [action, target, content]
  weights:
    action: 0.5
    target: 0.4
    content: 0.1
comparison:
  mode: min
  thresholds:
    action: 0.80
    target: 0.85
    content: 0.70
```

Another family (e.g., L2 output safety) might express:

```yaml
family: L2_OUTPUT_SAFETY
facets:
  use: [content, context]
  weights:
    content: 0.7
    context: 0.3
comparison:
  mode: weighted_mean
  thresholds:
    content: 0.90
    context: 0.75
```

The **comparison engine** is generic:

- Reads the config for the family.
- Pulls the relevant slices from the 128-dim intent/rule vectors.
- Applies the chosen aggregation (min/max/weighted mean/etc.).
- Compares against thresholds and produces a decision + slice-level evidence.

No per-family encoder logic is required to add or adjust a rule family; we adjust **config**, not encoding code.

---

## 3. Anchors as Data, Not Code

Rule anchors should be driven by **templates and data**, not custom code paths per family.

### 3.1 Anchor Templates Per Rule Type

Each rule type gets declarative anchor templates, e.g.:

```yaml
family: L4_TOOL_GATEWAY
rule_type: TOOL_WHITELIST
anchor_templates:
  allow:
    - "Safe use of tool {{ tool_name }} to {{ purpose }} for {{ audience }}"
  deny:
    - "Disallowed use of {{ tool_name }} to access {{ sensitive_resource }}"
```

The LLM anchor generator:

- Receives a `RuleInstance` plus `RuleFamilyConfig`.
- Renders anchors using templates and rule fields.
- Sends anchors through the same **facet → 128-dim encoder** as intents.

### 3.2 Adding New Families

To add a new family later, we largely:

- Define new templates and configs.
- Avoid touching the core encoder.
- Keep rule-specific language and thresholds as **data**.

---

## 4. Cross-Layer Decision Aggregation

Once all layers live in the same semantic space and use the same mechanics, the aggregation logic remains simple and generic.

Each rule evaluation returns a structured result like:

```json
{
  "family": "L4_TOOL_GATEWAY",
  "slice_scores": {
    "action": 0.93,
    "resource": 0.88,
    "data": 0.74,
    "risk": 0.40
  },
  "decision": "ALLOW",
  "evidence": [...]
}
```

The bridge’s `EnforcementEngine` applies a **layer evaluation strategy**, expressed as configuration:

- **Fail-closed**: if any mandatory family / blocking rule says `DENY`, short-circuit and block.
- **Advisory**: some layers contribute risk scores/metadata only (no hard block).

The aggregation logic:

- Works solely over scores, decisions, and metadata.
- Does **not** change encoding behavior.

---

## 5. Where Bespoke Logic Still Makes Sense

We keep a **narrow, explicit surface** for non-semantic or family-specific behavior, separate from encoding:

1. **Structural / non-semantic rules**
   - e.g., “no more than 3 tools per plan,” “no network tools after 22:00 local time.”
   - Implemented as straightforward checks on the `IntentEvent`, outside the semantic sandbox.

2. **Routing / applicability**
   - e.g., “apply L4 ToolGateway only when `tool_name` is set.”
   - This is rule applicability logic, not encoding logic.

3. **Extra scalar constraints**
   - e.g., numeric thresholds for cost, token count, rate limits.
   - Represented as additional structured fields on rules/events; combined with semantic scores in Rust, without altering the 128-dim vector layout.

By strictly separating these concerns, we avoid the drift toward “L7 has its own half-custom encoder” as the system grows.

---

## 6. Concrete Path From L4-Only to Multi-Layer

The current system is L4-focused. The path to multi-layer encoding within this design looks like:

1. **Refactor L4 to go through facets first**
   - Move current tool-aware intent encoding into `build_facets(IntentEvent)` so `tool_name`, `tool_method`, and `tool_params` enrich the **action** and **target/resource** facets.
   - Keep slice semantics aligned with the existing 128-dim layout.

2. **Express L4 thresholds as config**
   - Represent L4 ToolGateway thresholds and slice importance as configuration (even if currently expressed as in-code constants).

3. **Add families incrementally via config**
   - As new layers (L1/L2/L3) are implemented, define their facet usage + thresholds in config, and implement anchors via templates.
   - Keep the core encoding and comparison engine unchanged.

4. **Generic comparison and aggregation**
   - Ensure Rust comparison functions and the EnforcementEngine operate only on generic “family metadata + scores,” not on per-family special cases.

---

## 7. Relationship to Near-Term L4 Fix

This document captures the **target architecture** for multi-layer semantic encoding and comparison.

Near term, we will implement a **minimal fix for L4 similarity mismatches** (e.g., whitelisted tool scenarios like `whitelisted_tool_short_query` being blocked) that:

- Adjusts only L4-related facets/anchors/thresholds.
- Leaves the overall 128-dim layout and projection logic intact.
- Moves L4 closer to the facet-based model described above (where practical) without committing to a full refactor.

When we later optimize and generalize to all rule families, this architecture should allow us to:

- Reuse the minimal L4 fixes as a correctly aligned special case of the generic approach.
- Avoid rewriting the encoder yet again for additional layers.

---

## 8. Next Steps (For Future Optimization)

These steps are explicitly **deferred** until after the L4-only similarity mismatch is resolved:

1. Introduce a `build_facets(IntentEvent)` implementation in the Management Plane encoder consistent with this design.
2. Define a first-cut `RuleFamilyConfig` data model (Python + Rust) to express facet usage, thresholds, and aggregation modes.
3. Migrate the L4 ToolGateway family to use `RuleFamilyConfig` instead of hard-coded thresholds.
4. Add template-driven anchor definitions per rule type, consumed by the existing LLM anchor generator.
5. Update the Rust semantic sandbox and EnforcementEngine to accept generic family metadata and apply comparisons/aggregation accordingly.

This document should be treated as the **north star** for reducing tech debt as more rule families come online.

