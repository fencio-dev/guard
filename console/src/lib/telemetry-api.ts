/**
 * Telemetry API Client
 *
 * Connects to Management Plane HTTP API to fetch enforcement session data
 * from Data Plane hitlogs.
 */

import { supabase } from '@/lib/supabase';

// Management Plane URL - use relative path in production (proxied by nginx)
// In development, use full URL with port
const MANAGEMENT_PLANE_URL = import.meta.env.VITE_MANAGEMENT_PLANE_URL ||
  (import.meta.env.DEV ? 'http://localhost:8000' : '');

/**
 * Get authentication headers for Management Plane requests
 */
async function getAuthHeaders(): Promise<Headers> {
  const {
    data: { session },
  } = await supabase.auth.getSession();

  const token = session?.access_token;
  const userId = session?.user?.id;

  if (!token || !userId) {
    throw new Error('Could not validate credentials');
  }

  const headers = new Headers();
  headers.set('Authorization', `Bearer ${token}`);
  headers.set('X-Tenant-Id', userId);
  return headers;
}

/**
 * Session summary for list view
 * Maps to Management Plane's SessionSummary Pydantic model
 */
export interface SessionSummary {
    session_id: string;
    agent_id: string;
    tenant_id: string;
    layer: string;
    timestamp_ms: number;        // unix timestamp ms
    final_decision: 0 | 1;       // 0=BLOCK, 1=ALLOW
    rules_evaluated_count: number;
    duration_us: number;
    intent_summary: string;      // e.g., "postgres_query" or "write"
}

/**
 * Paginated response for session queries
 * Maps to Management Plane's TelemetrySessionsResponse Pydantic model
 */
export interface TelemetrySessionsResponse {
    sessions: SessionSummary[];
    totalCount: number;
    limit: number;
    offset: number;
}

/**
 * Full session detail with all enforcement data
 * Maps to Management Plane's SessionDetail Pydantic model
 */
export interface SessionDetail {
    session: {
        session_id: string;
        agent_id: string;
        tenant_id: string;
        layer: string;
        timestamp_ms: number;
        final_decision: 0 | 1;
        intent: any;              // Intent event JSON
        rules_evaluated: Array<{
            rule_id: string;
            rule_family: string;
            decision: number;
            slice_similarities: number[];
        }>;
        events?: Array<{
            type: string;
            timestamp_us: number;
            [key: string]: any;
        }>;
        performance?: {
            encoding_duration_us: number;
            rule_query_duration_us: number;
            evaluation_duration_us: number;
        };
        duration_us: number;
        [key: string]: any;       // Allow additional fields
    };
}

/**
 * Query parameters for fetching agent runs
 */
export interface FetchAgentRunsParams {
    agentId?: string;
    tenantId?: string;
    decision?: 0 | 1;         // 0=BLOCK, 1=ALLOW
    layer?: string;           // L0-L6
    startTime?: number;       // unix timestamp ms
    endTime?: number;         // unix timestamp ms
    limit?: number;           // default 50, max 500
    offset?: number;          // for pagination
}

/**
 * Fetch paginated list of enforcement sessions
 *
 * @param params - Query filters and pagination
 * @returns Paginated session summaries
 * @throws Error if API request fails
 */
export async function fetchAgentRuns(
    params: FetchAgentRunsParams = {}
): Promise<TelemetrySessionsResponse> {
    const queryParams = new URLSearchParams();

    // Add filters (use snake_case for API)
    if (params.agentId) queryParams.set('agent_id', params.agentId);
    if (params.tenantId) queryParams.set('tenant_id', params.tenantId);
    if (params.decision !== undefined) queryParams.set('decision', params.decision.toString());
    if (params.layer) queryParams.set('layer', params.layer);
    if (params.startTime) queryParams.set('start_time_ms', params.startTime.toString());
    if (params.endTime) queryParams.set('end_time_ms', params.endTime.toString());
    if (params.limit) queryParams.set('limit', params.limit.toString());
    if (params.offset) queryParams.set('offset', params.offset.toString());

    const url = `${MANAGEMENT_PLANE_URL}/api/v1/telemetry/sessions?${queryParams}`;

    const headers = await getAuthHeaders();
    const response = await fetch(url, { headers });

    if (!response.ok) {
        let errorMessage = `Telemetry query failed: ${response.statusText}`;
        try {
            const errorData = await response.json();
            errorMessage = errorData.detail || errorMessage;
        } catch {
            // Ignore JSON parse error
        }
        throw new Error(errorMessage);
    }

    return response.json();
}

/**
 * Fetch full details for a specific enforcement session
 *
 * @param sessionId - Unique session identifier
 * @returns Complete session data with rules, events, and performance metrics
 * @throws Error if session not found or API request fails
 */
export async function fetchSessionDetail(sessionId: string): Promise<SessionDetail> {
    const url = `${MANAGEMENT_PLANE_URL}/api/v1/telemetry/sessions/${sessionId}`;

    const headers = await getAuthHeaders();
    const response = await fetch(url, { headers });

    if (!response.ok) {
        if (response.status === 404) {
            throw new Error(`Session not found: ${sessionId}`);
        }

        let errorMessage = `Session detail fetch failed: ${response.statusText}`;
        try {
            const errorData = await response.json();
            errorMessage = errorData.detail || errorMessage;
        } catch {
            // Ignore JSON parse error
        }
        throw new Error(errorMessage);
    }

    return response.json();
}
