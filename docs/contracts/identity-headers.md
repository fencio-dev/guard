# Identity Header Contract

This document defines the internal identity header contract used across Tupl services.

## Header Definitions

### X-Tenant-Id (Required)
- **Value**: Supabase `auth.users.id` (UUID format)
- **Source**: Extracted from validated Supabase JWT by the calling service
- **Purpose**: Identifies the tenant/user for all backend operations
- **Example**: `X-Tenant-Id: 550e8400-e29b-41d4-a716-446655440000`

### X-User-Id (Optional)
- **Value**: Supabase `auth.users.id` (same as X-Tenant-Id in current single-user model)
- **Purpose**: Reserved for future multi-user-per-tenant scenarios
- **Example**: `X-User-Id: 550e8400-e29b-41d4-a716-446655440000`

## Service Communication Flows

### guard.fencio.dev → Management Plane
- **Authentication**: None (trusted via nginx internal routing)
- **Required Headers**: `X-Tenant-Id` (extracted from validated JWT)
- **Optional Headers**: `X-User-Id`

### guard.fencio.dev → Data Plane
- **Authentication**: None (trusted via nginx internal routing)
- **Required Headers**: `X-Tenant-Id` (forwarded by nginx)

### SDK → Management Plane
- **Authentication**: API key from `api_keys` table (existing flow, unchanged)
- **Headers**: `Authorization: Bearer <api_key>`

## Developer Platform Integration

### developer.fencio.dev → guard.fencio.dev Redirect
- **Method**: URL parameters
- **Format**: `https://guard.fencio.dev?token=<supabase_jwt>&api_key=<user_api_key>`
- **JWT Validation**: guard.fencio.dev validates JWT directly with Supabase
- **API Key**: Pre-populated in UI for user convenience

## Trust Model

- **External Requests**: All authentication happens at the edge (developer.fencio.dev or SDK with API keys)
- **Internal Requests**: Services trust nginx internal routing, no additional authentication required
- **Tenant Isolation**: Enforced via `X-Tenant-Id` header passed to all backend operations