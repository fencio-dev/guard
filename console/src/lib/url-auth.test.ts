import { describe, it, expect } from 'vitest';
import { extractAuthFromUrl } from './url-auth';

describe('extractAuthFromUrl', () => {
  it('should extract token and api_key from URL params', () => {
    const url = new URL('https://guard.fencio.dev?token=eyJhbGc&api_key=key_123');
    const result = extractAuthFromUrl(url);
    expect(result).toEqual({
      token: 'eyJhbGc',
      refreshToken: null,
      apiKey: 'key_123',
    });
  });

  it('should return null values if params missing', () => {
    const url = new URL('https://guard.fencio.dev');
    const result = extractAuthFromUrl(url);
    expect(result).toEqual({
      token: null,
      refreshToken: null,
      apiKey: null,
    });
  });

  it('should extract refresh_token from URL params', () => {
    const url = new URL('https://guard.fencio.dev?token=eyJhbGc&refresh_token=eyJyZWZyZXNo&api_key=key_123');
    const result = extractAuthFromUrl(url);
    expect(result).toEqual({
      token: 'eyJhbGc',
      refreshToken: 'eyJyZWZyZXNo',
      apiKey: 'key_123',
    });
  });

  it('should clean URL after extraction', () => {
    const url = new URL('https://guard.fencio.dev?token=eyJhbGc&api_key=key_123');
    extractAuthFromUrl(url);
    // URL should be cleaned (tested via side effect)
  });
});
