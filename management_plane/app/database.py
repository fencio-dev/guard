"""Supabase database helpers used across Management Plane services."""

from __future__ import annotations

import logging
import os
from typing import Any, Optional

try:
    from supabase import Client, create_client
except ImportError as exc:  # pragma: no cover - triggers only without dep
    raise RuntimeError(
        "supabase-py is required for database access. Install with `uv add supabase`."
    ) from exc

logger = logging.getLogger(__name__)


class SupabaseDB:
    """Thin CRUD wrapper around the Supabase Python client."""

    def __init__(self) -> None:
        supabase_url = os.getenv("SUPABASE_URL")
        supabase_key = os.getenv("SUPABASE_SERVICE_KEY") or os.getenv("SUPABASE_ANON_KEY")

        if not supabase_url or not supabase_key:
            raise ValueError(
                "SUPABASE_URL and SUPABASE_SERVICE_KEY (or SUPABASE_ANON_KEY) must be set"
            )

        self.client: Client = create_client(supabase_url, supabase_key)
        logger.debug("Supabase client initialized for %s", supabase_url)

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------
    @staticmethod
    def _get_rows(response: Any) -> list[dict[str, Any]]:
        if getattr(response, "error", None):
            raise RuntimeError(f"Supabase query failed: {response.error}")
        return response.data or []

    @staticmethod
    def _get_count(response: Any) -> int:
        if getattr(response, "error", None):
            raise RuntimeError(f"Supabase query failed: {response.error}")
        return int(getattr(response, "count", 0) or 0)

    # ------------------------------------------------------------------
    # CRUD operations
    # ------------------------------------------------------------------
    def select(self, table: str, columns: str = "*", **filters: Any) -> list[dict[str, Any]]:
        query = self.client.table(table).select(columns)

        if "eq" in filters:
            for col, val in filters["eq"].items():
                query = query.eq(col, val)

        if "limit" in filters:
            query = query.limit(filters["limit"])

        if "offset" in filters:
            query = query.offset(filters["offset"])

        if "order" in filters:
            query = query.order(filters["order"], desc=filters.get("desc", False))

        return self._get_rows(query.execute())

    def insert(self, table: str, data: dict[str, Any] | list[dict[str, Any]]) -> list[dict[str, Any]]:
        return self._get_rows(self.client.table(table).insert(data).execute())

    def update(self, table: str, data: dict[str, Any], **filters: Any) -> list[dict[str, Any]]:
        query = self.client.table(table).update(data)

        if "eq" in filters:
            for col, val in filters["eq"].items():
                query = query.eq(col, val)

        return self._get_rows(query.execute())

    def upsert(self, table: str, data: dict[str, Any] | list[dict[str, Any]]) -> list[dict[str, Any]]:
        return self._get_rows(self.client.table(table).upsert(data).execute())

    def delete(self, table: str, **filters: Any) -> list[dict[str, Any]]:
        query = self.client.table(table).delete()

        if "eq" in filters:
            for col, val in filters["eq"].items():
                query = query.eq(col, val)

        return self._get_rows(query.execute())

    def count(self, table: str, **filters: Any) -> int:
        query = self.client.table(table).select("*", count="exact")

        if "eq" in filters:
            for col, val in filters["eq"].items():
                query = query.eq(col, val)

        return self._get_count(query.execute())


# Global database instance
_db: Optional[SupabaseDB] = None


def get_db() -> SupabaseDB:
    """Return a cached SupabaseDB instance."""
    global _db
    if _db is None:
        _db = SupabaseDB()
    return _db
