-- Migration: Fix RLS Policy for User Tokens
-- Created: 2025-11-20
-- Purpose: Allow service role (gateway) to query user_tokens table for authentication
-- Issue: Previous RLS policy blocked service key queries, causing gateway timeouts

-- Problem: The existing RLS policy only allows authenticated users to see their own tokens
-- This blocks the MCP Gateway (which uses service_role key) from querying tokens
-- Result: Gateway queries hang/timeout because RLS blocks the SELECT

-- Solution: Add a policy that allows service_role to SELECT all tokens
-- Service role bypasses RLS by default, but explicit policy is good practice

-- Drop existing policies to recreate them
DROP POLICY IF EXISTS "Users can view their own token" ON user_tokens;
DROP POLICY IF EXISTS "Users can update their own token" ON user_tokens;
DROP POLICY IF EXISTS "Service role can query tokens for authentication" ON user_tokens;

-- Policy 1: Users can view their own token (authenticated users only)
CREATE POLICY "Users can view their own token"
  ON user_tokens
  FOR SELECT
  TO authenticated
  USING (auth.uid() = user_id);

-- Policy 2: Users can update their own token (for regeneration)
CREATE POLICY "Users can update their own token"
  ON user_tokens
  FOR UPDATE
  TO authenticated
  USING (auth.uid() = user_id);

-- Policy 3: Service role can query all tokens (for gateway authentication)
-- This policy allows the MCP Gateway to validate tokens
CREATE POLICY "Service role can query tokens for authentication"
  ON user_tokens
  FOR SELECT
  TO service_role
  USING (true);

-- Policy 4: Service role can update last_used_at timestamp
CREATE POLICY "Service role can update token usage"
  ON user_tokens
  FOR UPDATE
  TO service_role
  USING (true)
  WITH CHECK (true);

-- Grant permissions to service_role
GRANT SELECT, UPDATE ON user_tokens TO service_role;

-- Verify RLS is still enabled
ALTER TABLE user_tokens ENABLE ROW LEVEL SECURITY;

-- Add comment explaining the service role policy
COMMENT ON POLICY "Service role can query tokens for authentication" ON user_tokens IS
  'Allows MCP Gateway (using service_role key) to validate user tokens for authentication';

COMMENT ON POLICY "Service role can update token usage" ON user_tokens IS
  'Allows MCP Gateway to update last_used_at timestamp when token is used';
