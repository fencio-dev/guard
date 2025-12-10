-- Add registered_agents table
CREATE TABLE registered_agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sdk_version TEXT,
    metadata JSONB DEFAULT '{}',
    UNIQUE(tenant_id, agent_id)
);

CREATE INDEX idx_registered_agents_tenant_id ON registered_agents(tenant_id);
CREATE INDEX idx_registered_agents_tenant_agent ON registered_agents(tenant_id, agent_id);

ALTER TABLE registered_agents ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view their own agents"
    ON registered_agents FOR SELECT
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can register their own agents"
    ON registered_agents FOR INSERT
    WITH CHECK (auth.uid() = tenant_id);

-- Add agent_policies table
CREATE TABLE agent_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    agent_id TEXT NOT NULL,
    template_id TEXT NOT NULL,
    template_text TEXT NOT NULL,
    customization TEXT,
    policy_rules JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (tenant_id, agent_id)
        REFERENCES registered_agents(tenant_id, agent_id)
        ON DELETE CASCADE,
    UNIQUE(tenant_id, agent_id)
);

CREATE INDEX idx_agent_policies_tenant_id ON agent_policies(tenant_id);
CREATE INDEX idx_agent_policies_agent_id ON agent_policies(agent_id);
CREATE INDEX idx_agent_policies_tenant_agent ON agent_policies(tenant_id, agent_id);

ALTER TABLE agent_policies ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Users can view their own agent policies"
    ON agent_policies FOR SELECT
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can insert their own agent policies"
    ON agent_policies FOR INSERT
    WITH CHECK (auth.uid() = tenant_id);

CREATE POLICY "Users can update their own agent policies"
    ON agent_policies FOR UPDATE
    USING (auth.uid() = tenant_id);

CREATE POLICY "Users can delete their own agent policies"
    ON agent_policies FOR DELETE
    USING (auth.uid() = tenant_id);
