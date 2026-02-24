import { getAuthHeaders } from './headers';

const BASE = '/api/v2/telemetry';

export async function fetchSessions({ limit = 50, offset = 0 } = {}) {
  const url = `${BASE}/sessions?limit=${limit}&offset=${offset}`;
  const res = await fetch(url, { headers: getAuthHeaders() });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchSessionDetail(sessionId) {
  const res = await fetch(`${BASE}/sessions/${sessionId}`, { headers: getAuthHeaders() });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
