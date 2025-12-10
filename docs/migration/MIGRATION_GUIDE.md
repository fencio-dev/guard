# Supabase Key Migration Guide

## Overview

Supabase has migrated from legacy JWT-based authentication to a new API key model. This guide explains the changes required for this codebase.

## What Changed in Supabase

### Old Model (Legacy)
- **Anon Key**: JWT token (`eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...`)
- **Service Role Key**: UUID-like string
- **JWT Secret**: HS256 secret for JWT validation

### New Model (Current)
- **Publishable Key**: `sb_publishable_...` (replaces anon key)
- **Secret Key**: `sb_secret_...` (replaces service role key)
- **JWT Validation**: RS256 via JWKS endpoint (JWT secret deprecated)

## Environment Variable Updates

### Backend (Management Plane)

Update `deployment/.env`:

```bash
# Supabase Configuration
SUPABASE_URL=https://azkrxuiqcpxmsgydlyun.supabase.co
SUPABASE_SERVICE_KEY=sb_secret_YOUR_NEW_SECRET_KEY_HERE
# SUPABASE_JWT_SECRET is deprecated - remove or keep for backward compatibility
```

**Note**: Check your Supabase dashboard under Settings > API to see if JWT Secret is still available under "Legacy API Keys". If not, we need to migrate JWT validation to use JWKS.

User Notes: Yes, it's still available.

### Frontend (Console)

Update `console/.env`:

```bash
# Supabase Configuration
VITE_SUPABASE_URL=https://azkrxuiqcpxmsgydlyun.supabase.co
VITE_SUPABASE_ANON_KEY=sb_publishable_aGj614k3HmSwQw_B00lndA_BQnYY64_
```

## Code Changes Required

### Option 1: If JWT Secret Still Available (Backward Compatible)

If Supabase still provides a JWT secret under "Legacy API Keys", no code changes needed:

1. Keep using `SUPABASE_JWT_SECRET` in `management_plane/app/auth.py`
2. Update environment variables as shown above
3. Test that JWT validation still works

### Option 2: Migrate to JWKS/RS256 (Recommended)

If JWT secret is gone, update `management_plane/app/auth.py`:

User notes: The JWT Secret is still available, but I prefer Option 2. Let's go with this.

**Current code (line 141-168) uses HS256:**
```python
if not SUPABASE_JWT_SECRET:
    raise RuntimeError("SUPABASE_JWT_SECRET environment variable not set.")

try:
    payload = jwt.decode(
        token,
        SUPABASE_JWT_SECRET,
        algorithms=ALGORITHMS,  # ["HS256"]
        options={"verify_aud": False}
    )
```

**Needs to change to RS256 with JWKS:**
```python
try:
    # Fetch JWKS (cached via lru_cache)
    jwks = get_jwks()

    # Decode JWT using RS256 with JWKS
    # This requires python-jose[cryptography] or PyJWT with cryptography
    payload = jwt.decode(
        token,
        jwks,
        algorithms=["RS256"],  # Changed from HS256
        options={"verify_aud": False}
    )
```

## Testing the Migration

### 1. Test Backend API Key Validation

```bash
cd management_plane
SUPABASE_URL=https://azkrxuiqcpxmsgydlyun.supabase.co \
SUPABASE_SERVICE_KEY=sb_secret_YOUR_SECRET_KEY \
pytest tests/test_auth.py -v
```

### 2. Test Frontend Authentication

```bash
cd console
# Update .env with new keys
npm run dev
# Try logging in via Google OAuth
```

### 3. Test End-to-End Flow

```bash
# Start full stack
cd deployment
./deploy-local.sh

# Make authenticated request
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     http://localhost:8000/api/v1/agents
```

## Rollback Plan

If issues occur:

1. **Backend**: Revert to old service role key (if still valid)
2. **Frontend**: Revert to old anon key (if still valid)
3. **JWT Validation**: Keep using `SUPABASE_JWT_SECRET` if available

## Next Steps

1. ✅ Revoke old Gemini API key (completed)
2. ✅ Migrate Supabase keys (in progress)
3. ⏳ Update .env files with new keys
4. ⏳ Test authentication flow
5. ⏳ Update .env.example files
6. ⏳ Clean git history
7. ⏳ Push to public repo