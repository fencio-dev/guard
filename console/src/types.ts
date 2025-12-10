export type TransportType = 'stdio' | 'sse';

export interface StdioTransport {
  type: 'stdio';
  command: string;
  args?: string[];
  env?: Record<string, string>;
}

export interface SSETransport {
  type: 'sse';
  url: string;
  headers?: Record<string, string>;
}

export type Transport = StdioTransport | SSETransport;

export interface McpServer {
  id: string;
  label: string;
  transport: Transport;
  enabled?: boolean;
  credentials?: Record<string, string>;
}

// ============================================================================
// Policy Control Plane Types
// ============================================================================

export interface RuleFamilyConfig {
  enabled: boolean;
  priority?: number;
  params: Record<string, any>;
}

// L4 - Tool Gateway Layer Rule Families

export interface ToolWhitelistConfig extends RuleFamilyConfig {
  params: {
    allowed_tool_ids: string[];
    allowed_methods?: string[];
    rate_limit_per_min?: number;
    action: "DENY" | "ALLOW";
  };
}

export interface ToolParamConstraintConfig extends RuleFamilyConfig {
  params: {
    tool_id?: string;
    param_name: string;
    param_type: "string" | "number" | "boolean" | "integer";
    regex?: string;
    allowed_values?: any[];
    min_value?: number;
    max_value?: number;
    max_len?: number;
    enforcement_mode: "HARD" | "SOFT";
    allowed_methods: string[];
  };
}

export interface AgentRuleFamilies {
  // L0 - System
  sidecar_spawn?: RuleFamilyConfig;
  net_egress?: RuleFamilyConfig;

  // L1 - Input
  input_schema?: RuleFamilyConfig;
  input_sanitize?: RuleFamilyConfig;

  // L2 - Planner
  prompt_assembly?: RuleFamilyConfig;
  prompt_length?: RuleFamilyConfig;

  // L3 - Model I/O
  model_output_scan?: RuleFamilyConfig;
  model_output_escalate?: RuleFamilyConfig;

  // L4 - Tool Gateway
  tool_whitelist?: ToolWhitelistConfig;
  tool_param_constraints?: ToolParamConstraintConfig[];

  // L5 - RAG
  rag_source?: RuleFamilyConfig;
  rag_doc_sensitivity?: RuleFamilyConfig;

  // L6 - Egress
  output_pii?: RuleFamilyConfig;
  output_audit?: RuleFamilyConfig;
}

export interface AgentProfile {
  agent_id: string;
  owner: string;
  description?: string;
  rule_families: AgentRuleFamilies;
  rollout_mode: "immediate" | "staged" | "canary";
  canary_pct: number;
  metadata: Record<string, string>;
}

export interface RuleConfigResponse {
  config_id: string;
  agent_id: string;
  owner: string;
  created_at: string;
  rule_count: number;
  rules_by_layer: Record<string, number>;
  rules?: any[];
}

export interface RuleConfigListResponse {

  total: number;

  configurations: RuleConfigResponse[];

}



export interface ApiKey {

  key: string;

  keyPrefix: string;

  description?: string;

  createdAt: string;

  lastUsed?: string;

  expiresAt?: string;

}

// ============================================================================
// Natural Language Guardrails Types
// ============================================================================

export interface RegisteredAgentSummary {
  id: string;
  agent_id: string;
  first_seen: string;
  last_seen: string;
  sdk_version?: string;
}

export interface ListRegisteredAgentsResponse {
  total: number;
  agents: RegisteredAgentSummary[];
}

export interface PolicyTemplate {
  id: string;
  name: string;
  description: string;
  template_text: string;
  category: string;
  example_customizations: string[];
}

export interface ListTemplatesResponse {
  templates: PolicyTemplate[];
}

export interface AgentPolicyRecord {
  id: string;
  agent_id: string;
  template_id: string;
  template_text: string;
  customization?: string;
  policy_rules: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}
