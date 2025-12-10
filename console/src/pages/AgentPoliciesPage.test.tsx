import { describe, expect, it, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AgentPoliciesPage from './AgentPoliciesPage';
import type { PolicyTemplate } from '@/types';

const mockAgents = {
  total: 1,
  agents: [
    {
      id: '1',
      agent_id: 'agent-alpha',
      first_seen: new Date().toISOString(),
      last_seen: new Date().toISOString(),
      sdk_version: '1.0.0',
    },
  ],
};

const templates: PolicyTemplate[] = [
  {
    id: 'db_read',
    name: 'DB Template',
    description: 'Read databases',
    template_text: 'Allow reading',
    category: 'database',
    example_customizations: ['only analytics'],
  },
  {
    id: 'file_export',
    name: 'File Export',
    description: 'Export files',
    template_text: 'Allow export',
    category: 'file',
    example_customizations: ['csv only'],
  },
];

const agentApiMocks = vi.hoisted(() => ({
  listRegisteredAgents: vi.fn(),
  listTemplates: vi.fn(),
  getAgentPolicy: vi.fn(),
  createAgentPolicy: vi.fn(),
}));

vi.mock('@/lib/agent-api', () => agentApiMocks);

const {
  listRegisteredAgents,
  listTemplates,
  getAgentPolicy,
  createAgentPolicy,
} = agentApiMocks;

const renderPage = () => render(<AgentPoliciesPage />);

beforeEach(() => {
  vi.resetAllMocks();
  listRegisteredAgents.mockResolvedValue(mockAgents);
  listTemplates.mockResolvedValue({ templates });
  getAgentPolicy.mockResolvedValue(null);
  createAgentPolicy.mockResolvedValue({ id: 'policy-1' });
});

describe('AgentPoliciesPage', () => {
  it('loads agents and templates on mount', async () => {
    renderPage();

    await waitFor(() => expect(listRegisteredAgents).toHaveBeenCalled());
    await waitFor(() => expect(listTemplates).toHaveBeenCalled());

    await expect(screen.findByText('DB Template')).resolves.toBeInTheDocument();
  });

  it('fetches agent policy after selection and allows creation', async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText('DB Template');
    await waitFor(() => expect(getAgentPolicy).toHaveBeenCalledWith('agent-alpha'));

    await user.click(screen.getByRole('button', { name: /DB Template/i }));
    await user.type(screen.getByLabelText(/Customize/i), 'limit to staging');
    await user.click(screen.getByRole('button', { name: /Create Policy/i }));

    expect(createAgentPolicy).toHaveBeenCalledWith({
      agent_id: 'agent-alpha',
      template_id: 'db_read',
      template_text: 'Allow reading',
      customization: 'limit to staging',
    });
  });

  it('filters templates by category tabs', async () => {
    const user = userEvent.setup();
    renderPage();

    await screen.findByText('DB Template');

    expect(screen.getByText('DB Template')).toBeInTheDocument();
    expect(screen.getByText('File Export')).toBeInTheDocument();

    await user.click(screen.getByRole('tab', { name: /file/i }));

    expect(screen.queryByText('DB Template')).not.toBeInTheDocument();
    expect(screen.getByText('File Export')).toBeInTheDocument();
  });
});
