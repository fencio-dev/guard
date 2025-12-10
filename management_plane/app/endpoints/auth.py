"""
Authentication endpoints.

JWT validation now happens at the edge (developer.fencio.dev or guard.fencio.dev).
Management Plane trusts X-Tenant-Id headers from guard.fencio.dev via nginx internal routing.
"""

import logging
from fastapi import APIRouter

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/auth", tags=["auth"])

# No endpoints currently needed.
# Authentication flows:
# 1. guard.fencio.dev → MP: Uses X-Tenant-Id header (trusted via nginx)
# 2. SDK → MP: Uses JWT Bearer token (validated by get_current_user)
