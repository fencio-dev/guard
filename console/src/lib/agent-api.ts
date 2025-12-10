import { supabase } from '@/lib/supabase';
import type {
  AgentPolicyRecord,
  ListRegisteredAgentsResponse,
  ListTemplatesResponse,
} from '@/types';

const API_BASE =
  import.meta.env.VITE_MANAGEMENT_PLANE_URL ||
  (import.meta.env.DEV ? 'http://localhost:8000' : '');

async function getAuthHeaders(extra?: HeadersInit): Promise<Headers> {
  const {
    data: { session },
  } = await supabase.auth.getSession();

  const token = session?.access_token;
  const userId = session?.user?.id;

  if (!token || !userId) {
    throw new Error('Missing Supabase session. Please sign in again.');
  }

  const headers = new Headers(extra);
  headers.set('Authorization', `Bearer ${token}`);
  headers.set('X-Tenant-Id', userId);
  return headers;
}

async function authedFetch(path: string, init: RequestInit = {}) {
  const headers = await getAuthHeaders(init.headers);
  if (init.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  return fetch(`${API_BASE}/api/v1${path}`, {
    ...init,
    headers,
  });
}

async function handleResponse<T>(response: Response, errorMessage: string): Promise<T> {
  if (!response.ok) {
    try {
      const details = await response.json();
      const message =
        (typeof details === 'string' && details) || details?.detail || details?.message;
      if (message) {
        throw new Error(`${errorMessage}: ${message}`);
      }
    } catch (err) {
      if (err instanceof Error && err.message.startsWith(errorMessage)) {
        throw err;
      }
    }
    throw new Error(errorMessage);
  }

  if (response.status === 204) {
    return {} as T;
  }

  return response.json() as Promise<T>;
}

export interface CreateAgentPolicyPayload {
  agent_id: string;
  template_id: string;
  template_text: string;
  customization?: string;
}

export async function listRegisteredAgents(): Promise<ListRegisteredAgentsResponse> {
  const response = await authedFetch('/agents/list');
  return handleResponse(response, 'Failed to fetch agents');
}

export async function listTemplates(category?: string): Promise<ListTemplatesResponse> {
  const query = category ? `?category=${encodeURIComponent(category)}` : '';
  const response = await authedFetch(`/agents/templates${query}`);
  return handleResponse(response, 'Failed to fetch templates');
}

export async function createAgentPolicy(data: CreateAgentPolicyPayload): Promise<AgentPolicyRecord> {
  const response = await authedFetch('/agents/policies', {
    method: 'POST',
    body: JSON.stringify(data),
  });
  return handleResponse(response, 'Failed to create policy');
}

export async function getAgentPolicy(agentId: string): Promise<AgentPolicyRecord | null> {
  const response = await authedFetch(`/agents/policies/${agentId}`);
  if (response.status === 404) {
    return null;
  }
  return handleResponse(response, 'Failed to fetch policy');
}

export async function deleteAgentPolicy(agentId: string): Promise<void> {
  const response = await authedFetch(`/agents/policies/${agentId}`, {
    method: 'DELETE',
  });
  await handleResponse(response, 'Failed to delete policy');
}
