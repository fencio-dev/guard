import React, { createContext, useContext, useEffect, useState } from 'react';
import type { Session, User } from '@supabase/supabase-js';
import { supabase } from '../lib/supabase';
import { extractAuthFromUrl } from '../lib/url-auth';

interface AuthContextType {
  session: Session | null;
  user: User | null;
  loading: boolean;
  apiKey: string | null;
  setApiKey: (key: string | null) => void;
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [session, setSession] = useState<Session | null>(null);
  const [user, setUser] = useState<User | null>(null);
  const [apiKey, setApiKeyState] = useState<string | null>(() => {
    return localStorage.getItem('tupl-api-key');
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const startTime = Date.now();
    console.log('AuthContext: Initializing...', { timestamp: startTime });

    // Check for auth params from developer platform redirect
    const { token: urlToken, refreshToken: urlRefreshToken, apiKey: urlApiKey } = extractAuthFromUrl();

    // Only set session if we have BOTH access and refresh tokens
    if (urlToken && urlRefreshToken) {
      console.log('AuthContext: Found tokens in URL params, setting session');
      // Set session from URL tokens
      supabase.auth.setSession({
        access_token: urlToken,
        refresh_token: urlRefreshToken,
      }).then(({ data: { session }, error }) => {
        if (error) {
          console.error('AuthContext: Failed to set session from URL tokens', error);
          setLoading(false);
          return;
        }
        console.log('AuthContext: Session set successfully from URL tokens');
        setSession(session);
        setUser(session?.user ?? null);

        // Set API key from URL if provided
        if (urlApiKey) {
          setApiKey(urlApiKey);
        }
        setLoading(false);
      });
    } else {
      // Fall back to retrieving existing session from storage
      supabase.auth.getSession().then(({ data: { session } }) => {
        const elapsed = Date.now() - startTime;
        console.log('AuthContext: Retrieved session from storage', {
          hasSession: session ? 'Session found' : 'No session',
          elapsed: `${elapsed}ms`,
        });
        setSession(session);
        setUser(session?.user ?? null);
        setLoading(false);
      });
    }

    // Listen for auth changes
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((event, session) => {
      console.log('AuthContext: onAuthStateChange', { event });
      setSession(session);
      setUser(session?.user ?? null);
      if (!session) {
        setApiKey(null);
      }
    });

    return () => subscription.unsubscribe();
  }, []);

  const setApiKey = (key: string | null) => {
    if (key) {
      localStorage.setItem('tupl-api-key', key);
    } else {
      localStorage.removeItem('tupl-api-key');
    }
    setApiKeyState(key);
  };

  const signIn = async () => {
    const { error } = await supabase.auth.signInWithOAuth({
      provider: 'google',
      options: {
        redirectTo: `${window.location.origin}/auth/callback`,
      },
    });
    if (error) throw error;
  };

  const signOut = async () => {
    const { error } = await supabase.auth.signOut();
    if (error) throw error;
  };

  return (
    <AuthContext.Provider
      value={{ session, user, loading, apiKey, setApiKey, signIn, signOut }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
