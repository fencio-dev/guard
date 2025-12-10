export interface ApiKey {
  id: string;
  name: string;
  keyPreview: string;
  created: Date;
  lastUsed: Date | null;
}

export const mockApiKeys: ApiKey[] = [
  {
    id: 'key_1',
    name: 'Production Server',
    keyPreview: 'console_sk_1a2b3c4d...',
    created: new Date('2025-11-01T10:00:00'),
    lastUsed: new Date(Date.now() - 2 * 60 * 1000) // 2 minutes ago
  },
  {
    id: 'key_2',
    name: 'Development Environment',
    keyPreview: 'console_sk_5e6f7g8h...',
    created: new Date('2025-11-10T14:30:00'),
    lastUsed: new Date(Date.now() - 30 * 60 * 1000) // 30 minutes ago
  },
  {
    id: 'key_3',
    name: 'Testing & Staging',
    keyPreview: 'console_sk_9i0j1k2l...',
    created: new Date('2025-11-15T09:15:00'),
    lastUsed: null
  }
];

export function generateApiKey(): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let key = 'console_sk_';
  for (let i = 0; i < 32; i++) {
    key += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return key;
}
