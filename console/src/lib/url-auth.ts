/**
 * Extracts authentication parameters from URL query string.
 * Used when redirecting from developer.fencio.dev to guard.fencio.dev.
 */
export interface UrlAuthParams {
  token: string | null;
  refreshToken: string | null;
  apiKey: string | null;
}

export function extractAuthFromUrl(url: URL = new URL(window.location.href)): UrlAuthParams {
  const params = new URLSearchParams(url.search);
  const token = params.get('token');
  const refreshToken = params.get('refresh_token');
  const apiKey = params.get('api_key');

  // Clean URL by removing auth params
  if (token || refreshToken || apiKey) {
    params.delete('token');
    params.delete('refresh_token');
    params.delete('api_key');
    const newUrl = `${url.pathname}${params.toString() ? `?${params.toString()}` : ''}${url.hash}`;
    window.history.replaceState({}, '', newUrl);
  }

  return { token, refreshToken, apiKey };
}
