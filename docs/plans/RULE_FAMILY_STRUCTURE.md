# Rule Family Structure Reference

## Overview

This document clarifies the **exact structure** that rule families must follow when configuring agents.

## Correct Structure

### When using `tupl_configure_agent_rules`:

```json
{
  "agent_id": "my-agent",
  "owner": "security-team",
  "rule_families": {
    "input_sanitize": {
      "enabled": true,
      "priority": 1,
      "params": {
        "strip_fields": [],
        "allowed_fields": ["email"],
        "max_depth": 10,
        "normalize_unicode": true,
        "action": "DENY"
      }
    },
    "output_pii": {
      "enabled": true,
      "priority": 1,
      "params": {
        "semantic_hook": "pii-detector-v1",
        "action": "REDACT",
        "redact_template": "[REDACTED]",
        "pii_types": ["EMAIL"],
        "max_exec_ms": 500,
        "confidence_threshold": 0.75
      }
    }
  }
}
```

## Using Output from `tupl_generate_rules_from_nlp`

When you call `tupl_generate_rules_from_nlp`, the response looks like:

```json
{
  "rule_families": {
    "input_sanitize": { ... },
    "output_pii": { ... }
  },
  "reasoning": "...",
  "model_used": "gemini-2.5-flash-lite",
  "status": "success"
}
```

### ⚠️ Important: How to Use This Output

**Copy ONLY the `rule_families` value** (the inner object), NOT the whole response:

```json
// ✅ CORRECT - Copy this part:
{
  "input_sanitize": { ... },
  "output_pii": { ... }
}

// ❌ WRONG - Don't include the wrapper:
{
  "rule_families": {
    "input_sanitize": { ... },
    "output_pii": { ... }
  }
}
```

## Rule Family Configuration Schema

Each rule family follows this structure:

```typescript
{
  "<rule_family_id>": {
    "enabled": boolean,        // Optional, defaults to true
    "priority": number,         // Optional, uses family default if not set
    "params": {
      // Family-specific parameters
      // See schemas/rule_families.py for exact fields per family
    }
  }
}
```

## Available Rule Families

### L0 - System Layer
- `net_egress` - Network egress control
- `sidecar_spawn` - Sidecar spawn restrictions

### L1 - Input Layer
- `input_schema` - Input schema validation
- `input_sanitize` - Input sanitization

### L2 - Planner Layer
- `prompt_assembly` - Prompt assembly control
- `prompt_length` - Prompt length limiting

### L3 - Model I/O Layer
- `model_output_scan` - Model output scanning
- `model_output_escalate` - Model output escalation

### L4 - Tool Gateway Layer
- `tool_whitelist` - Tool whitelisting
- `tool_param_constraint` - Tool parameter constraints

### L5 - RAG Layer
- `rag_source` - RAG source restrictions
- `rag_doc_sensitivity` - RAG document sensitivity filtering

### L6 - Egress Layer
- `output_pii` - Output PII detection/redaction
- `output_audit` - Output auditing

## Complete Parameter Schemas

See [mcp-tupl-server/src/mcp_tupl/schemas/rule_families.py](mcp-tupl-server/src/mcp_tupl/schemas/rule_families.py) for the complete, definitive parameter schemas for each rule family.

### Example: `input_sanitize`

```json
{
  "input_sanitize": {
    "enabled": true,
    "priority": 850,
    "params": {
      "strip_fields": [],                    // Array of field names to remove
      "allowed_fields": null,                // Array of allowed fields (whitelist mode)
      "max_depth": 10,                       // Maximum nesting depth
      "normalize_unicode": true,             // Normalize Unicode characters
      "action": "REWRITE"                    // "REWRITE" or "DENY"
    }
  }
}
```

### Example: `output_pii`

```json
{
  "output_pii": {
    "enabled": true,
    "priority": 950,
    "params": {
      "semantic_hook": "pii-detector-v1",   // WASM hook reference
      "action": "REDACT",                    // "REDACT" or "DENY"
      "redact_template": "[REDACTED]",      // Replacement string
      "pii_types": ["PII", "SSN", "CREDIT_CARD", "EMAIL"],  // Types to detect
      "max_exec_ms": 40,                     // Max execution time
      "confidence_threshold": 0.6            // Detection threshold (0.0-1.0)
    }
  }
}
```

## Workflow Example

1. **Generate rules from NLP:**
   ```json
   {
     "tool": "tupl_generate_rules_from_nlp",
     "arguments": {
       "nlp_description": "Allow only email addresses in input and output"
     }
   }
   ```

2. **Response:**
   ```json
   {
     "rule_families": {
       "input_sanitize": { "enabled": true, "priority": 1, "params": {...} },
       "output_pii": { "enabled": true, "priority": 1, "params": {...} }
     },
     "reasoning": "...",
     "model_used": "gemini-2.5-flash-lite",
     "status": "success"
   }
   ```

3. **Copy the inner `rule_families` value and use it:**
   ```json
   {
     "tool": "tupl_configure_agent_rules",
     "arguments": {
       "agent_id": "my-agent",
       "owner": "team",
       "rule_families": {
         "input_sanitize": { "enabled": true, "priority": 1, "params": {...} },
         "output_pii": { "enabled": true, "priority": 1, "params": {...} }
       }
     }
   }
   ```

## Validation

The Control Plane validates:
- Rule family IDs exist in the catalog
- Required parameters are present
- Parameter types match schema
- Enum values are valid
- Numeric ranges are within bounds

Invalid configurations will be rejected with detailed error messages.
