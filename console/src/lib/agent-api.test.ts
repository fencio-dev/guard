import { beforeEach, describe, expect, it, vi } from 'vitest';

const getSessionMock = vi.fn();

vi.mock('@/lib/supabase', () => ({
  supabase: {
    auth: {
      getSession: getSessionMock,
    },
  },
}));

// Lazy import to ensure mocks are applied.
const agentApi = () => import('./agent-api');

const mockFetch = vi.fn();

function jsonResponse<T>(data: T, init: Partial<Response> = {}) {
  return {
    ok: init.ok ?? true,
    status: init.status ?? 200,
    statusText: init.statusText ?? 'OK',
    json: vi.fn().mockResolvedValue(data),
  } as unknown as Response;
}

beforeEach(() => {
  getSessionMock.mockResolvedValue({
    data: { session: { access_token: 'test-token' } },
  });
  mockFetch.mockReset();
  // @ts-expect-error - test shim
  global.fetch = mockFetch;
});

describe('agent-api', () => {
  it('attaches bearer token when listing registered agents', async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse({ total: 1, agents: [] }));

    const { listRegisteredAgents } = await agentApi();
    const result = await listRegisteredAgents();

    expect(result).toEqual({ total: 1, agents: [] });
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/agents/list'),
      expect.any(Object)
    );

    const [, options] = mockFetch.mock.calls[0];
    expect(options?.headers?.get('Authorization')).toBe('Bearer test-token');
  });

  it('throws a descriptive error when templates request fails', async () => {
    mockFetch.mockResolvedValueOnce(
      jsonResponse(
        { detail: 'boom' },
        { ok: false, status: 500, statusText: 'Server Error' }
      )
    );

    const { listTemplates } = await agentApi();

    await expect(listTemplates()).rejects.toThrow(/Failed to fetch templates/);
  });

  it('passes category filters to templates endpoint', async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse({ templates: [] }));

    const { listTemplates } = await agentApi();
    await listTemplates('database');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('category=database'),
      expect.any(Object)
    );
  });

  it('creates agent policies with template payload', async () => {
    const responseBody = { id: 'policy-1' };
    mockFetch.mockResolvedValueOnce(jsonResponse(responseBody));

    const { createAgentPolicy } = await agentApi();
    const payload = {
      agent_id: 'agent-1',
      template_id: 'template-1',
      template_text: 'Allow X',
      customization: 'only read',
    };

    const result = await createAgentPolicy(payload);

    expect(result).toEqual(responseBody);
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/agents/policies'),
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify(payload),
      })
    );
  });

  it('returns null when agent policy is not found', async () => {
    mockFetch.mockResolvedValueOnce(
      jsonResponse(
        { detail: 'not found' },
        { ok: false, status: 404, statusText: 'Not Found' }
      )
    );

    const { getAgentPolicy } = await agentApi();
    const policy = await getAgentPolicy('missing-agent');

    expect(policy).toBeNull();
  });

  it('deletes policies via API', async () => {
    mockFetch.mockResolvedValueOnce(jsonResponse({ success: true }));

    const { deleteAgentPolicy } = await agentApi();
    await deleteAgentPolicy('agent-123');

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/agents/policies/agent-123'),
      expect.objectContaining({ method: 'DELETE' })
    );
  });
});
