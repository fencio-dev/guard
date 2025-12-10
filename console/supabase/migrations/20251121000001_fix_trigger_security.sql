-- Migration: Fix User Token Trigger to Bypass RLS
-- Created: 2025-11-21
-- Purpose: Allow trigger to insert tokens for new users by using SECURITY DEFINER
-- Issue: Trigger fails because it lacks INSERT permission, blocking new user signups

-- Problem Analysis:
-- 1. The trigger on_user_created runs after INSERT on auth.users
-- 2. It calls generate_user_token() which tries to INSERT into user_tokens
-- 3. RLS is enabled on user_tokens with no INSERT policy
-- 4. Trigger fails with "Database error saving new user"
-- 5. User signup rolls back completely

-- Solution:
-- Make generate_user_token() run with SECURITY DEFINER so it bypasses RLS
-- This is safe because:
-- - Function only inserts token for the NEW user (can't insert for other users)
-- - ON CONFLICT DO NOTHING prevents duplicates
-- - No user input, fully controlled logic

-- Recreate the function with SECURITY DEFINER
CREATE OR REPLACE FUNCTION generate_user_token()
RETURNS TRIGGER
SECURITY DEFINER  -- Run as function owner (postgres), bypassing RLS
SET search_path = public
AS $$
BEGIN
  -- Insert token for the newly created user
  -- ON CONFLICT ensures idempotency (safe to retry)
  INSERT INTO user_tokens (user_id, token)
  VALUES (NEW.id, generate_unique_token())
  ON CONFLICT (user_id) DO NOTHING;

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add comment explaining security model
COMMENT ON FUNCTION generate_user_token() IS
  'Trigger function to auto-generate tokens for new users.
   Uses SECURITY DEFINER to bypass RLS (safe because it only inserts for NEW.id).
   Called by on_user_created trigger on auth.users table.';
