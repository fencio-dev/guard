import type { AgentProfile, RuleConfigResponse, RuleConfigListResponse } from '@/types';

// Policy Control Plane URL (port 8001)
// In production, this service is not yet deployed - features will be unavailable
// In development, use localhost:8001
const CONTROL_PLANE_BASE_URL = import.meta.env.VITE_CONTROL_PLANE_URL ||
  (import.meta.env.DEV ? 'http://localhost:8001' : null);

async function request<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  // Control Plane service not available - return empty/null response
  if (!CONTROL_PLANE_BASE_URL) {
    console.warn('Control Plane service not available - returning empty response');
    if (path.includes('/rules')) {
      return { rules: [], count: 0 } as T;
    }
    return null as T;
  }

  const headers = new Headers(options.headers);
  // Add Supabase token here if/when the Control Plane supports JWT auth
  // headers.set('Authorization', `Bearer ${token}`);

  if (options.body) {
    headers.set('Content-Type', 'application/json');
  }

  const response = await fetch(`${CONTROL_PLANE_BASE_URL}${path}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({}));
    throw new Error(
      `API request failed with status ${response.status}: ${
        errorData.detail || response.statusText
      }`
    );
  }

  if (response.status === 204) {
    return null as T;
  }

  return response.json() as T;
}

export const controlPlaneApi = {
  listConfigs: (agentId?: string): Promise<RuleConfigListResponse> => {
    const query = agentId ? `?agent_id=${encodeURIComponent(agentId)}` : '';
    return request(`/api/v1/rules${query}`, { method: 'GET' });
  },

  getConfig: (agentId: string): Promise<RuleConfigResponse> => {
    return request(`/api/v1/agents/${agentId}/rules`, { method: 'GET' });
  },

  saveConfig: (agentId: string, profile: AgentProfile): Promise<RuleConfigResponse> => {
    return request(`/api/v1/agents/${agentId}/rules`, {
      method: 'POST',
      body: JSON.stringify({ profile }),
    });
  },

  deleteConfig: (agentId: string): Promise<void> => {
    return request(`/api/v1/agents/${agentId}/rules`, { method: 'DELETE' });
  },
};
