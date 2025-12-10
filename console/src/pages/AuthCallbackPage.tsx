import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';

export default function AuthCallbackPage() {
  const { user, loading } = useAuth();
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Parse URL parameters
    const hash = window.location.hash;
    const search = window.location.search;
    const href = window.location.href;

    console.log('AuthCallbackPage: URL details', {
      href,
      hash: hash || '(empty)',
      search: search || '(empty)',
      pathname: window.location.pathname
    });

    // Check for OAuth error in URL (hash or search params)
    const hashParams = new URLSearchParams(hash.substring(1)); // Remove leading #
    const searchParams = new URLSearchParams(search);

    const errorParam = hashParams.get('error') || searchParams.get('error');
    const errorDescription = hashParams.get('error_description') || searchParams.get('error_description');
    const errorCode = hashParams.get('error_code') || searchParams.get('error_code');

    // If there's an OAuth error, display it immediately
    if (errorParam) {
      const errorMessage = errorDescription
        ? decodeURIComponent(errorDescription.replace(/\+/g, ' '))
        : errorParam;

      console.error('AuthCallbackPage: OAuth error', { errorParam, errorCode, errorDescription });
      setError(errorMessage);
      return; // Stay on page to show error
    }

    // Check for success auth parameters
    const hasSuccessParams =
      hash.includes('access_token') ||
      hash.includes('type=recovery') ||
      search.includes('code');

    console.log('AuthCallbackPage: checking', {
      loading,
      user: user ? 'User found' : 'No user',
      hasSuccessParams,
      hasError: !!errorParam
    });

    if (loading) return;

    // If user is authenticated, redirect to home
    if (user) {
      console.log('AuthCallbackPage: Redirecting to /');
      navigate('/', { replace: true });
      return;
    }

    // If no success params and no error, redirect to login
    if (!hasSuccessParams) {
      console.log('AuthCallbackPage: Redirecting to /login (no auth params)');
      navigate('/login', { replace: true });
      return;
    }

    // If we have success params but no user yet, wait for Supabase to process
    console.log('AuthCallbackPage: Waiting for Supabase to process auth params...');
    const timer = setTimeout(() => {
      if (!user) {
        console.log('AuthCallbackPage: Auth timeout, redirecting to /login');
        setError('Authentication timeout. Please try again.');
      }
    }, 5000); // 5 seconds timeout

    return () => clearTimeout(timer);
  }, [user, loading, navigate]);

  // Show error state if there's an error
  if (error) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background">
        <div className="text-center space-y-4 max-w-md p-6">
          <div className="rounded-full h-12 w-12 bg-destructive/10 mx-auto flex items-center justify-center">
            <svg className="h-6 w-6 text-destructive" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </div>
          <h2 className="text-lg font-semibold text-foreground">Authentication Failed</h2>
          <p className="text-sm text-muted-foreground">{error}</p>
          <button
            onClick={() => navigate('/login', { replace: true })}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
          >
            Back to Login
          </button>
        </div>
      </div>
    );
  }

  // Show loading state while processing authentication
  return (
    <div className="flex min-h-screen items-center justify-center bg-background">
      <div className="text-center space-y-4">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
        <h2 className="text-lg font-medium text-foreground">Completing sign in...</h2>
      </div>
    </div>
  );
}
