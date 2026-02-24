/**
 * Returns auth headers for all API calls.
 * Guard's get_current_tenant checks X-Tenant-Id first — no JWT needed for local use.
 */
export function getAuthHeaders(extra = {}) {
  const tenantId = localStorage.getItem('guardTenantId') || '';
  return {
    'X-Tenant-Id': tenantId,
    ...extra,
  };
}
