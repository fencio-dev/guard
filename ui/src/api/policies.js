import { getAuthHeaders } from './headers';

export async function fetchPolicies() {
  const res = await fetch('/api/v2/policies', { headers: getAuthHeaders() });
  if (!res.ok) throw new Error(`Failed to fetch policies: ${res.status}`);
  const data = await res.json();
  return data.policies;
}

export async function deletePolicy(id) {
  const res = await fetch(`/api/v2/policies/${id}`, {
    method: 'DELETE',
    headers: getAuthHeaders(),
  });
  if (!res.ok) throw new Error(`Failed to delete policy ${id}: ${res.status}`);
}

export async function createPolicy(data) {
  const res = await fetch('/api/v2/policies', {
    method: 'POST',
    headers: getAuthHeaders({ 'Content-Type': 'application/json' }),
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error(`Failed to create policy: ${res.status}`);
  return res.json();
}
