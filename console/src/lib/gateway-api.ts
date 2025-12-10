import type { McpServer, ApiKey } from '@/types';

const BASE_URL = import.meta.env.VITE_GATEWAY_BASE_URL || 'http://localhost:3000';

async function request<T>(endpoint: string, token: string, options: RequestInit = {}): Promise<T> {
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${token}`,
    ...options.headers,
  };

  const response = await fetch(`${BASE_URL}/api${endpoint}`, {
    ...options,
    headers,
  });

  if (!response.ok) {
    let errorMessage = 'API request failed';
    try {
      const errorData = await response.json();
      errorMessage = errorData.error || errorMessage;
    } catch {
      // ignore json parse error
    }
    throw new Error(errorMessage);
  }

  // Handle 204 No Content
  if (response.status === 204) {
    return {} as T;
  }

  return response.json();
}

export const gatewayApi = {
  // Server Management
  listServers: (token: string) => request<McpServer[]>('/servers', token),
  
  createServer: (token: string, server: McpServer) => 
    request<McpServer>('/servers', token, {
      method: 'POST',
      body: JSON.stringify(server),
    }),
    
  updateServer: (token: string, serverId: string, updates: Partial<McpServer>) => 
    request<McpServer>(`/servers/${serverId}`, token, {
      method: 'PUT',
      body: JSON.stringify(updates),
    }),
    
  deleteServer: (token: string, serverId: string) => 
    request<void>(`/servers/${serverId}`, token, {
      method: 'DELETE',
    }),

  // API Key Management
  listApiKeys: (token: string) => request<{ keys: ApiKey[] }>('/keys', token).then(res => res.keys),
  
  generateApiKey: (token: string, description: string) => 
    request<{ apiKey: ApiKey }>('/keys', token, {
      method: 'POST',
      body: JSON.stringify({ description }),
    }).then(res => res.apiKey),
    
  revokeApiKey: (token: string, key: string) => 
    request<void>(`/keys/${key}`, token, {
      method: 'DELETE',
    }),
};