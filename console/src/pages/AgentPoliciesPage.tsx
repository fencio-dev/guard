import { useEffect, useMemo, useState } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TemplateCard } from '@/components/TemplateCard';
import {
  createAgentPolicy,
  getAgentPolicy,
  listRegisteredAgents,
  listTemplates,
} from '@/lib/agent-api';
import type {
  AgentPolicyRecord,
  PolicyTemplate,
  RegisteredAgentSummary,
} from '@/types';
import { Loader2 } from 'lucide-react';

const CATEGORY_FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'database', label: 'Database' },
  { value: 'file', label: 'File' },
  { value: 'api', label: 'API' },
  { value: 'general', label: 'General' },
];

const AgentPoliciesPage = () => {
  const [agents, setAgents] = useState<RegisteredAgentSummary[]>([]);
  const [templates, setTemplates] = useState<PolicyTemplate[]>([]);
  const [selectedAgent, setSelectedAgent] = useState('');
  const [selectedTemplateId, setSelectedTemplateId] = useState<string | null>(null);
  const [customization, setCustomization] = useState('');
  const [currentPolicy, setCurrentPolicy] = useState<AgentPolicyRecord | null>(null);
  const [category, setCategory] = useState('all');
  const [error, setError] = useState<string | null>(null);
  const [loadingAgents, setLoadingAgents] = useState(true);
  const [loadingTemplates, setLoadingTemplates] = useState(true);
  const [creatingPolicy, setCreatingPolicy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function loadAgents() {
      try {
        setLoadingAgents(true);
        const response = await listRegisteredAgents();
        if (!cancelled) {
          const agentList = response?.agents ?? [];
          setAgents(agentList);
          if (!selectedAgent && agentList.length > 0) {
            setSelectedAgent(agentList[0].agent_id);
          }
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Failed to load agents', err);
          setError('Failed to load agents');
        }
      } finally {
        if (!cancelled) {
          setLoadingAgents(false);
        }
      }
    }
    loadAgents();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function loadTemplatesList() {
      try {
        setLoadingTemplates(true);
        const response = await listTemplates();
        if (!cancelled) {
          setTemplates(response?.templates ?? []);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Failed to load templates', err);
          setError('Failed to load templates');
        }
      } finally {
        if (!cancelled) {
          setLoadingTemplates(false);
        }
      }
    }
    loadTemplatesList();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!selectedAgent) {
      setCurrentPolicy(null);
      return;
    }

    let cancelled = false;
    async function fetchPolicy(agentId: string) {
      try {
        const policy = await getAgentPolicy(agentId);
        if (!cancelled) {
          setCurrentPolicy(policy);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Failed to fetch policy', err);
          setError('Failed to fetch policy');
          setCurrentPolicy(null);
        }
      }
    }

    fetchPolicy(selectedAgent);
    return () => {
      cancelled = true;
    };
  }, [selectedAgent]);

  const filteredTemplates = useMemo(() => {
    if (category === 'all') {
      return templates;
    }
    return templates.filter((template) => template.category === category);
  }, [category, templates]);

  const selectedTemplate = useMemo(
    () => templates.find((template) => template.id === selectedTemplateId) ?? null,
    [selectedTemplateId, templates]
  );

  const handleTemplateSelect = (template: PolicyTemplate) => {
    setSelectedTemplateId(template.id);
    setCustomization('');
  };

  const handleCreatePolicy = async () => {
    if (!selectedAgent || !selectedTemplate) {
      return;
    }

    setCreatingPolicy(true);
    setError(null);
    try {
      await createAgentPolicy({
        agent_id: selectedAgent,
        template_id: selectedTemplate.id,
        template_text: selectedTemplate.template_text,
        customization: customization || undefined,
      });

      setCustomization('');
      setSelectedTemplateId(null);
      await getAgentPolicy(selectedAgent).then(setCurrentPolicy);
    } catch (err) {
      console.error('Failed to create policy', err);
      setError((err as Error)?.message || 'Failed to create policy');
    } finally {
      setCreatingPolicy(false);
    }
  };

  const agentLabelId = 'agent-select-label';
  const canCreatePolicy = Boolean(selectedAgent && selectedTemplate && !creatingPolicy);

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Agent Policies</h1>
        <p className="text-muted-foreground">
          Configure natural language security guardrails for any registered agent.
        </p>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="space-y-2">
        <Label id={agentLabelId}>Select Agent</Label>
        <Select value={selectedAgent} onValueChange={(value) => setSelectedAgent(value)}>
          <SelectTrigger aria-labelledby={agentLabelId} aria-label="Select Agent">
            <SelectValue placeholder={loadingAgents ? 'Loading agents…' : 'Choose an agent'} />
          </SelectTrigger>
          <SelectContent>
            {loadingAgents && (
              <div className="px-4 py-2 text-sm text-muted-foreground">Loading…</div>
            )}
            {!loadingAgents && agents.length === 0 && (
              <div className="px-4 py-2 text-sm text-muted-foreground">No agents found</div>
            )}
            {agents.map((agent) => (
              <SelectItem key={agent.id} value={agent.agent_id}>
                <div className="flex flex-col">
                  <span className="font-medium">{agent.agent_id}</span>
                  <span className="text-xs text-muted-foreground">
                    Last seen {new Date(agent.last_seen).toLocaleString()}
                  </span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {selectedAgent && currentPolicy && (
        <Alert>
          <AlertDescription>
            Current policy template <strong>{currentPolicy.template_id}</strong>
            {currentPolicy.customization && ` – ${currentPolicy.customization}`}
          </AlertDescription>
        </Alert>
      )}

      {selectedAgent ? (
        <>
          <Tabs value={category} onValueChange={setCategory} className="w-full">
            <TabsList>
              {CATEGORY_FILTERS.map((tab) => (
                <TabsTrigger key={tab.value} value={tab.value} role="tab">
                  {tab.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          {loadingTemplates ? (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" /> Loading templates…
            </div>
          ) : filteredTemplates.length === 0 ? (
            <div className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
              No templates available for this category.
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {filteredTemplates.map((template) => (
                <TemplateCard
                  key={template.id}
                  template={template}
                  selected={selectedTemplateId === template.id}
                  onSelect={() => handleTemplateSelect(template)}
                />
              ))}
            </div>
          )}

          {selectedTemplate && (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="policy-customization">Customize (Optional)</Label>
                <Textarea
                  id="policy-customization"
                  value={customization}
                  onChange={(event) => setCustomization(event.target.value)}
                  placeholder={
                    selectedTemplate.example_customizations?.[0] || 'Add natural language customization…'
                  }
                  rows={4}
                />
              </div>
              <Button
                className="w-full"
                onClick={handleCreatePolicy}
                disabled={!canCreatePolicy}
              >
                {creatingPolicy ? 'Creating Policy…' : 'Create Policy'}
              </Button>
            </div>
          )}
        </>
      ) : (
        <div className="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
          Select an agent to browse available templates.
        </div>
      )}
    </div>
  );
};

export default AgentPoliciesPage;
