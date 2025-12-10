# Control Plane to Data Plane Integration Guide

This document describes the integration between the Policy Control Plane and the TUPL Data Plane, explaining how rules created in the control plane are installed into the data plane's bridge tables.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    POLICY CONTROL PLANE                      │
│                      (Python/FastAPI)                        │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  1. User Creates Agent Profile with Rule Families     │ │
│  │     POST /api/v1/agents/{agent_id}/rules              │ │
│  │                                                         │ │
│  │  2. Compiler Generates Rule Instances                  │ │
│  │     - Converts rule family configs to rule instances   │ │
│  │     - Assigns priorities, validates parameters         │ │
│  │                                                         │ │
│  │  3. Data Plane Client Pushes Rules via gRPC           │ │
│  │     - Serializes rules to protobuf format              │ │
│  │     - Calls gRPC InstallRules service                  │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      │ gRPC (Protocol Buffers)
                      │ Port: 50051
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                      TUPL DATA PLANE                         │
│                         (Rust)                               │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  4. gRPC Server Receives Rules                         │ │
│  │     - Deserializes protobuf messages                    │ │
│  │     - Validates rule structure                          │ │
│  │                                                         │ │
│  │  5. Rule Conversion                                     │ │
│  │     - Converts from control plane format                │ │
│  │     - Creates native Rust rule instances               │ │
│  │     - Instantiates correct rule family types           │ │
│  │                                                         │ │
│  │  6. Bridge Installation                                 │ │
│  │     - Routes rules to appropriate family tables         │ │
│  │     - Builds indices for fast lookup                    │ │
│  │     - Updates table versions atomically                │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                               │
│  ┌────────────────────────────────────────────────────────┐ │
│  │                    BRIDGE (14 Tables)                   │ │
│  │                                                         │ │
│  │  L0: NetworkEgress, SidecarSpawn                       │ │
│  │  L1: InputSchema, InputSanitize                        │ │
│  │  L2: PromptAssembly, PromptLength                      │ │
│  │  L3: ModelOutputScan, ModelOutputEscalate              │ │
│  │  L4: ToolWhitelist, ToolParamConstraint                │ │
│  │  L5: RAGSource, RAGDocSensitivity                      │ │
│  │  L6: OutputPII, OutputAudit                            │ │
│  │                                                         │ │
│  │  ✓ Lock-free reads (sub-microsecond)                  │ │
│  │  ✓ Per-family indices (agent, tool, source, domain)   │ │
│  │  ✓ Priority-based evaluation                           │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Control Plane Components

#### 1.1 Models (`policy_control_plane/models.py`)
- **AgentProfile**: User-defined agent configuration
- **AgentRuleFamilies**: Container for all 14 rule family configs
- **RuleFamilyConfig**: Individual rule family configuration (one per family)

Each rule family has its own config class with family-specific parameters:
- `SidecarSpawnConfig`, `NetEgressConfig`
- `InputSchemaConfig`, `InputSanitizeConfig`
- `PromptAssemblyConfig`, `PromptLengthConfig`
- `ModelOutputScanConfig`, `ModelOutputEscalateConfig`
- `ToolWhitelistConfig`, `ToolParamConstraintConfig`
- `RAGSourceConfig`, `RAGDocSensitivityConfig`
- `OutputPIIConfig`, `OutputAuditConfig`

#### 1.2 Compiler (`policy_control_plane/compiler.py`)
- **RuleCompiler**: Converts AgentProfile to RuleInstance objects
- **RuleInstance**: Concrete rule ready for data plane installation

Compilation process:
1. Iterates through enabled rule families
2. Creates RuleInstance for each enabled family
3. Assigns priority based on defaults or overrides
4. Packages parameters for data plane

#### 1.3 Data Plane Client (`policy_control_plane/dataplane_client.py`)
- **DataPlaneClient**: gRPC client for communication with data plane
- Methods:
  - `install_rules()`: Push rules to data plane
  - `remove_agent_rules()`: Remove all rules for an agent
  - `get_rule_stats()`: Query current bridge statistics

#### 1.4 Server (`policy_control_plane/server.py`)
- FastAPI server exposing rule management endpoints
- After compiling rules, automatically pushes to data plane
- Stores data plane installation status

Key endpoints:
- `POST /api/v1/agents/{agent_id}/rules`: Create and install rules
- `GET /api/v1/agents/{agent_id}/rules`: Get agent rules
- `GET /api/v1/rules`: List all rule configurations

### 2. Data Plane Components

#### 2.1 Bridge (`tupl_data_plane/tupl_dp/bridge/src/bridge.rs`)
- **Bridge**: Root data structure with 14 rule family tables
- Thread-safe, lock-free reads
- Methods:
  - `add_rule()`: Add single rule to appropriate table
  - `add_rules_batch()`: Add multiple rules efficiently
  - `query_by_agent()`: Query rules for an agent
  - `stats()`: Get bridge statistics

#### 2.2 Rule Family Tables (`tupl_data_plane/tupl_dp/bridge/src/table.rs`)
- **RuleFamilyTable**: Stores rules for one family
- Per-family indexing strategies:
  - Agent index (all families)
  - Secondary indices (tool, source, domain, image)
- Lock-free reads via atomic Arc pointers
- Copy-on-write updates

#### 2.3 gRPC Server (`tupl_data_plane/tupl_dp/bridge/src/grpc_server.rs`)
- **RuleInstallationService**: gRPC service implementation
- Methods:
  - `install_rules()`: Receive and install rules from control plane
  - `remove_agent_rules()`: Remove all rules for an agent
  - `get_rule_stats()`: Return bridge statistics

Installation process:
1. Receive control plane rules (protobuf format)
2. Parse family_id to determine rule type
3. Convert parameters to native Rust types
4. Instantiate appropriate rule struct (e.g., ToolWhitelistRule)
5. Add to bridge, which routes to correct table
6. Return installation statistics

#### 2.4 Main Entry Point (`tupl_data_plane/tupl_dp/bridge/src/main.rs`)
- Initializes bridge with 14 empty tables
- Starts gRPC server on port 50051
- Displays bridge statistics
- Keeps server running

### 3. Protocol Buffers (`proto/rule_installation.proto`)

Defines gRPC service and message types:

```protobuf
service RuleInstallation {
  rpc InstallRules(InstallRulesRequest) returns (InstallRulesResponse);
  rpc RemoveAgentRules(RemoveAgentRulesRequest) returns (RemoveAgentRulesResponse);
  rpc GetRuleStats(GetRuleStatsRequest) returns (GetRuleStatsResponse);
}
```

Key message types:
- **RuleInstance**: Rule to be installed (family, layer, agent, params, priority)
- **ParamValue**: Polymorphic parameter value (string, int, float, bool, list)
- **InstallRulesResponse**: Installation results with statistics

## Data Flow

### Creating Rules for an Agent

```
User → Control Plane → Data Plane → Bridge Tables

1. User POSTs agent profile with rule family configs
   POST /api/v1/agents/agent_1/rules
   {
     "profile": {
       "agent_id": "agent_1",
       "owner": "security-team",
       "rule_families": {
         "tool_whitelist": {
           "enabled": true,
           "priority": 600,
           "params": {
             "allowed_tool_ids": ["postgres", "redis"],
             "rate_limit_per_min": 100
           }
         },
         "output_pii": {
           "enabled": true,
           "params": {
             "semantic_hook": "pii-detector-v1",
             "action": "REDACT"
           }
         }
       }
     }
   }

2. Control Plane Compiler generates RuleInstances
   - agent_1_tool_whitelist (family: tool_whitelist, layer: L4)
   - agent_1_output_pii (family: output_pii, layer: L6)

3. Data Plane Client serializes to protobuf and calls gRPC
   dataplane_client.install_rules(
     agent_id="agent_1",
     rules=[rule1, rule2],
     config_id="abc-123",
     owner="security-team"
   )

4. Data Plane gRPC Server receives request
   - Deserializes protobuf messages
   - Converts to Rust rule types:
     * ToolWhitelistRule with specified params
     * OutputPIIRule with specified params

5. Bridge routes rules to correct tables
   - ToolWhitelistRule → L4 ToolWhitelist table
   - OutputPIIRule → L6 OutputPII table

6. Tables update indices
   - Agent index: agent_1 → [rule1, rule2]
   - Tool index (for tool_whitelist): postgres, redis → [rule1]
   - Priority sorting: Descending order

7. Response flows back to control plane
   {
     "success": true,
     "rules_installed": 2,
     "rules_by_layer": {"L4": 1, "L6": 1},
     "bridge_version": 1
   }
```

### Querying Rules During Evaluation

```
Request → Evaluation Engine → Bridge Query → Rules

1. Agent request arrives at data plane
   - Agent: agent_1
   - Operation: Tool call to "postgres"

2. Evaluation engine queries bridge
   rules = bridge.query_by_agent(
     family_id=RuleFamilyId::ToolWhitelist,
     agent_id="agent_1"
   )

3. Bridge returns rules (lock-free, < 1μs)
   - Returns agent_1's ToolWhitelistRule
   - Plus any global ToolWhitelist rules
   - Sorted by priority (descending)

4. Evaluation engine processes rules
   for rule in rules:
     if rule.matches(context):
       execute(rule.action)
       break  # Short-circuit on first match

5. Rule is evaluated
   - Check: Is "postgres" in allowed_tool_ids? ✓ Yes
   - Action: ALLOW
   - Decision logged
```

## Rule Family Mapping

Control plane `family_id` maps to data plane `RuleFamilyId`:

| Control Plane | Data Plane | Layer | Secondary Index |
|--------------|------------|-------|-----------------|
| `net_egress` | `NetworkEgress` | L0 | Domain |
| `sidecar_spawn` | `SidecarSpawn` | L0 | Image |
| `input_schema` | `InputSchema` | L1 | None |
| `input_sanitize` | `InputSanitize` | L1 | None |
| `prompt_assembly` | `PromptAssembly` | L2 | None |
| `prompt_length` | `PromptLength` | L2 | None |
| `model_output_scan` | `ModelOutputScan` | L3 | None |
| `model_output_escalate` | `ModelOutputEscalate` | L3 | None |
| `tool_whitelist` | `ToolWhitelist` | L4 | Tool |
| `tool_param_constraint` | `ToolParamConstraint` | L4 | Tool |
| `rag_source` | `RAGSource` | L5 | Source |
| `rag_doc_sensitivity` | `RAGDocSensitivity` | L5 | Source |
| `output_pii` | `OutputPII` | L6 | None |
| `output_audit` | `OutputAudit` | L6 | None |

## Parameter Conversion

Control plane parameters (JSON) are converted to Rust types:

### Example: ToolWhitelistRule

**Control Plane (Python)**:
```python
{
  "allowed_tool_ids": ["postgres", "redis"],
  "allowed_methods": ["query", "get"],
  "rate_limit_per_min": 100
}
```

**Data Plane (Rust)**:
```rust
ToolWhitelistRule {
  allowed_tool_ids: vec!["postgres".to_string(), "redis".to_string()],
  allowed_methods: vec!["query".to_string(), "get".to_string()],
  rate_limit_per_min: Some(100),
  // ... other fields
}
```

### ParamValue Conversion

| Control Plane | Protobuf | Rust |
|--------------|----------|------|
| `"string"` | `string_value` | `String` |
| `42` | `int_value` | `i64` |
| `3.14` | `float_value` | `f64` |
| `true` | `bool_value` | `bool` |
| `["a", "b"]` | `string_list` | `Vec<String>` |

## Error Handling

### Control Plane Errors
- Agent ID mismatch
- Duplicate agent configuration
- Invalid rule family parameters
- Data plane connection failure (logged but doesn't fail request)

### Data Plane Errors
- Unknown family_id
- Invalid parameter types
- Rule conversion failures
- Bridge add_rule failures

## Performance Characteristics

### Control Plane
- Rule compilation: ~1-10ms for typical agent profile
- gRPC call: ~1-50ms depending on network
- Total latency: ~10-100ms for full workflow

### Data Plane
- Rule installation: ~10-100μs per rule
- Batch installation: ~1-10ms for 10 rules
- Query latency: < 1μs (lock-free reads)
- Throughput: 20M+ queries/second

### Scalability
- Rules per agent: Up to 100+ rules
- Agents: Thousands of agents
- Total rules in bridge: Millions (theoretical)
- Memory: ~100 bytes per rule

## Configuration

### Control Plane
```python
# Environment variables or config file
DATAPLANE_HOST = "localhost"  # or data plane hostname
DATAPLANE_PORT = 50051
```

### Data Plane
```rust
// Port for gRPC server
const GRPC_PORT: u16 = 50051;
```

## Deployment Considerations

### Development
1. Start data plane: `cd tupl_data_plane/tupl_dp/bridge && cargo run`
2. Start control plane: `cd policy_control_plane && python server.py`
3. Send requests to control plane: `http://localhost:8000`

### Production
1. Deploy data plane as a service (Docker/K8s)
2. Deploy control plane as a service
3. Configure control plane with data plane endpoint
4. Use TLS for gRPC communication
5. Add authentication/authorization
6. Implement rule versioning and rollback
7. Add monitoring and alerting

## Testing

### Integration Test Flow
```bash
# 1. Start data plane
cd tupl_data_plane/tupl_dp/bridge
cargo run

# 2. Start control plane
cd policy_control_plane
python server.py

# 3. Create agent with rules
curl -X POST http://localhost:8000/api/v1/agents/agent_1/rules \
  -H "Content-Type: application/json" \
  -d @sample_agent_profile.json

# 4. Verify rules installed
curl http://localhost:8000/api/v1/agents/agent_1/rules

# Expected response includes:
# - Rule instances
# - Data plane installation status
# - Bridge version
```

## Future Enhancements

1. **Rule Versioning**: Track rule versions for auditing and rollback
2. **Incremental Updates**: Update only changed rules instead of full replacement
3. **Rule Validation**: Pre-validate rules in control plane before sending
4. **Metrics & Monitoring**: Expose Prometheus metrics from data plane
5. **Hot Reload**: Support atomic rule set updates without downtime
6. **Rule Simulation**: Test rules without deploying to production
7. **A/B Testing**: Deploy rules to subset of traffic for testing
8. **Rule Templates**: Reusable rule templates for common patterns
9. **Conflict Detection**: Detect conflicting rules across families
10. **Performance Analytics**: Track rule evaluation performance

## Troubleshooting

### Rules not appearing in data plane
- Check control plane logs for gRPC errors
- Verify data plane gRPC server is running
- Check network connectivity between control and data plane

### Rule conversion errors
- Verify family_id matches expected values
- Check parameter types match rule family requirements
- Review data plane logs for conversion errors

### Query performance issues
- Check number of rules per agent (too many?)
- Verify indices are being used correctly
- Review bridge statistics

## References

- [Control Plane Models](policy_control_plane/models.py)
- [Control Plane Compiler](policy_control_plane/compiler.py)
- [Data Plane Bridge](tupl_data_plane/tupl_dp/bridge/src/bridge.rs)
- [gRPC Server](tupl_data_plane/tupl_dp/bridge/src/grpc_server.rs)
- [Protocol Buffers](proto/rule_installation.proto)
- [Bridge Architecture](tupl_data_plane/tupl_dp/ARCH_DIAGRAM.md)
- [Rule Families Reference](tupl_data_plane/tupl_dp/bridge/FAMILIES_REF.md)
