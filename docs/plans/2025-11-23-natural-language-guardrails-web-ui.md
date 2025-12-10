# Natural Language Guardrails - Web UI Implementation Plan

> **Goal:** Add Web UI for natural language policy configuration to complement existing MCP Gateway integration.

**Status**: Ready for Implementation
**Prerequisites**: Tasks 1-11 complete (backend APIs, SDK, MCP tools)
**Tech Stack**: React, TypeScript, Tailwind CSS, Supabase Auth

---

## Overview

Implement Web UI components for agent policy configuration using the existing Management Plane APIs. Users will browse templates, customize policies with natural language, and manage agent policies through a visual interface.

**Reference**: [Design Document](2025-11-22-natural-language-guardrails-design.md#user-experience)

---

## Task 1: API Client Library

**Files:**
- Create: `web-console/src/lib/agent-api.ts`

**Implementation:**

```typescript
// lib/agent-api.ts
import { supabase } from './supabase';

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

async function getAuthHeaders() {
  const { data: { session } } = await supabase.auth.getSession();
  return {
    'Authorization': `Bearer ${session?.access_token}`,
    'Content-Type': 'application/json'
  };
}

export async function listRegisteredAgents() {
  const headers = await getAuthHeaders();
  const response = await fetch(`${API_BASE}/api/v1/agents/list`, { headers });
  if (!response.ok) throw new Error('Failed to fetch agents');
  return response.json();
}

export async function listTemplates(category?: string) {
  const headers = await getAuthHeaders();
  const url = category
    ? `${API_BASE}/api/v1/agents/templates?category=${category}`
    : `${API_BASE}/api/v1/agents/templates`;
  const response = await fetch(url, { headers });
  if (!response.ok) throw new Error('Failed to fetch templates');
  return response.json();
}

export async function createAgentPolicy(data: {
  agent_id: string;
  template_id: string;
  template_text: string;
  customization?: string;
}) {
  const headers = await getAuthHeaders();
  const response = await fetch(`${API_BASE}/api/v1/agents/policies`, {
    method: 'POST',
    headers,
    body: JSON.stringify(data)
  });
  if (!response.ok) throw new Error('Failed to create policy');
  return response.json();
}

export async function getAgentPolicy(agentId: string) {
  const headers = await getAuthHeaders();
  const response = await fetch(`${API_BASE}/api/v1/agents/policies/${agentId}`, { headers });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error('Failed to fetch policy');
  return response.json();
}

export async function deleteAgentPolicy(agentId: string) {
  const headers = await getAuthHeaders();
  const response = await fetch(`${API_BASE}/api/v1/agents/policies/${agentId}`, {
    method: 'DELETE',
    headers
  });
  if (!response.ok) throw new Error('Failed to delete policy');
  return response.json();
}
```

**Verification:**
```bash
# No tests needed - thin wrapper around fetch
```

---

## Task 2: Template Card Component

**Files:**
- Create: `web-console/src/components/TemplateCard.tsx`

**Implementation:**

```typescript
// components/TemplateCard.tsx
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';

interface TemplateCardProps {
  template: {
    id: string;
    name: string;
    description: string;
    template_text: string;
    category: string;
    example_customizations: string[];
  };
  selected?: boolean;
  onSelect: () => void;
}

export function TemplateCard({ template, selected, onSelect }: TemplateCardProps) {
  return (
    <Card
      className={`cursor-pointer transition-all hover:shadow-lg ${
        selected ? 'ring-2 ring-primary' : ''
      }`}
      onClick={onSelect}
    >
      <CardHeader>
        <div className="flex items-start justify-between">
          <CardTitle className="text-lg">{template.name}</CardTitle>
          <Badge variant="outline">{template.category}</Badge>
        </div>
        <CardDescription>{template.description}</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <p className="text-sm font-medium">Template:</p>
          <p className="text-sm text-muted-foreground italic">"{template.template_text}"</p>

          {template.example_customizations.length > 0 && (
            <>
              <p className="text-sm font-medium pt-2">Example customizations:</p>
              <ul className="text-sm text-muted-foreground list-disc list-inside">
                {template.example_customizations.slice(0, 3).map((ex, i) => (
                  <li key={i}>{ex}</li>
                ))}
              </ul>
            </>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
```

**Verification:**
```bash
# Visual verification in Storybook or dev mode
```

---

## Task 3: Agent Policies Page

**Files:**
- Create: `web-console/src/app/(dashboard)/agent-policies/page.tsx`

**Implementation:**

```typescript
// app/(dashboard)/agent-policies/page.tsx
'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TemplateCard } from '@/components/TemplateCard';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  listRegisteredAgents,
  listTemplates,
  createAgentPolicy,
  getAgentPolicy
} from '@/lib/agent-api';

export default function AgentPoliciesPage() {
  const [agents, setAgents] = useState([]);
  const [templates, setTemplates] = useState([]);
  const [selectedAgent, setSelectedAgent] = useState('');
  const [selectedTemplate, setSelectedTemplate] = useState(null);
  const [customization, setCustomization] = useState('');
  const [currentPolicy, setCurrentPolicy] = useState(null);
  const [category, setCategory] = useState('all');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    loadAgents();
    loadTemplates();
  }, []);

  useEffect(() => {
    if (selectedAgent) {
      loadCurrentPolicy(selectedAgent);
    }
  }, [selectedAgent]);

  async function loadAgents() {
    try {
      const data = await listRegisteredAgents();
      setAgents(data.agents || []);
    } catch (err) {
      setError('Failed to load agents');
    }
  }

  async function loadTemplates() {
    try {
      const data = await listTemplates();
      setTemplates(data.templates || []);
    } catch (err) {
      setError('Failed to load templates');
    }
  }

  async function loadCurrentPolicy(agentId: string) {
    try {
      const policy = await getAgentPolicy(agentId);
      setCurrentPolicy(policy);
    } catch (err) {
      setCurrentPolicy(null);
    }
  }

  async function handleCreatePolicy() {
    if (!selectedAgent || !selectedTemplate) return;

    setLoading(true);
    setError('');

    try {
      await createAgentPolicy({
        agent_id: selectedAgent,
        template_id: selectedTemplate.id,
        template_text: selectedTemplate.template_text,
        customization: customization || undefined
      });

      await loadCurrentPolicy(selectedAgent);
      setSelectedTemplate(null);
      setCustomization('');
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  const filteredTemplates = category === 'all'
    ? templates
    : templates.filter(t => t.category === category);

  return (
    <div className="container mx-auto py-8 space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Agent Policies</h1>
        <p className="text-muted-foreground">Configure natural language security policies for your agents</p>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Agent Selection */}
      <div className="space-y-2">
        <Label htmlFor="agent-select">Select Agent</Label>
        <Select value={selectedAgent} onValueChange={setSelectedAgent}>
          <SelectTrigger id="agent-select">
            <SelectValue placeholder="Choose an agent..." />
          </SelectTrigger>
          <SelectContent>
            {agents.map(agent => (
              <SelectItem key={agent.agent_id} value={agent.agent_id}>
                {agent.agent_id} (last seen: {new Date(agent.last_seen).toLocaleString()})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {selectedAgent && currentPolicy && (
        <Alert>
          <AlertDescription>
            Current policy: {currentPolicy.template_id}
            {currentPolicy.customization && ` - ${currentPolicy.customization}`}
          </AlertDescription>
        </Alert>
      )}

      {selectedAgent && (
        <>
          {/* Category Filter */}
          <Tabs value={category} onValueChange={setCategory}>
            <TabsList>
              <TabsTrigger value="all">All</TabsTrigger>
              <TabsTrigger value="database">Database</TabsTrigger>
              <TabsTrigger value="file">File</TabsTrigger>
              <TabsTrigger value="api">API</TabsTrigger>
              <TabsTrigger value="general">General</TabsTrigger>
            </TabsList>
          </Tabs>

          {/* Template Grid */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredTemplates.map(template => (
              <TemplateCard
                key={template.id}
                template={template}
                selected={selectedTemplate?.id === template.id}
                onSelect={() => setSelectedTemplate(template)}
              />
            ))}
          </div>

          {/* Customization */}
          {selectedTemplate && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="customization">Customize (Optional)</Label>
                <Textarea
                  id="customization"
                  placeholder={selectedTemplate.example_customizations[0] || "Add natural language customization..."}
                  value={customization}
                  onChange={(e) => setCustomization(e.target.value)}
                  rows={3}
                />
              </div>

              <Button
                onClick={handleCreatePolicy}
                disabled={loading}
                className="w-full"
              >
                {loading ? 'Creating Policy...' : 'Create Policy'}
              </Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
```

**Verification:**
```bash
npm run dev
# Navigate to /agent-policies and test flow
```

---

## Task 4: Navigation Update

**Files:**
- Modify: `web-console/src/components/Navigation.tsx` (or equivalent)

**Implementation:**

Add navigation item:
```typescript
{
  name: 'Agent Policies',
  href: '/agent-policies',
  icon: ShieldCheckIcon, // or appropriate icon
}
```

**Verification:**
```bash
# Visual check - navigation item appears
```

---

## Summary

**Total Tasks**: 4
**New Files**: 3
**Modified Files**: 1
**Estimated Time**: 2-3 hours

**Key Features**:
- Agent selection dropdown (auto-populated)
- Template browsing with category filter
- Natural language customization input
- Policy preview and creation
- Current policy display

**Dependencies**:
- Existing shadcn/ui components (Card, Button, Select, etc.)
- Supabase auth integration
- Management Plane APIs (already implemented)

**Next Steps After Implementation**:
1. Test full flow: select agent → choose template → customize → create policy
2. Verify RLS policies work correctly (tenant isolation)
3. Test error cases (agent not registered, API failures)
4. Add to integration test suite

---

**Implementation Ready**: All required APIs exist. UI components follow existing patterns in web-console.
