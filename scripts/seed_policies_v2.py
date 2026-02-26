#!/usr/bin/env python3
"""Seed v2-compatible demo policies into SQLite + Chroma.

This script seeds 13 demo policies with explicit scoring_mode values so startup
sync and v2 validation pass on fresh environments.

Usage:
  python scripts/seed_policies_v2.py
  python scripts/seed_policies_v2.py --tenant-id demo-tenant
"""

from __future__ import annotations

import argparse
import os
import sys
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


PROJECT_ROOT = Path(__file__).resolve().parents[1]
MANAGEMENT_PLANE_ROOT = PROJECT_ROOT / "management_plane"

# Ensure local imports resolve as: from app...
sys.path.insert(0, str(MANAGEMENT_PLANE_ROOT))

# Pragmatic defaults for fresh local setup.
os.environ.setdefault("DATABASE_URL", f"sqlite:///{(MANAGEMENT_PLANE_ROOT / 'mgmt_plane.db').resolve()}")
os.environ.setdefault("CHROMA_URL", str((PROJECT_ROOT / "data" / "chroma_data").resolve()))

from app.models import (  # noqa: E402
    DesignBoundary,
    PolicyMatch,
    SliceThresholds,
    SliceWeights,
)
from app.services.policies import (  # noqa: E402
    build_anchor_payload,
    create_policy_record,
    fetch_policy_record,
    update_policy_record,
    upsert_policy_payload,
)
from app.services.policy_encoder import PolicyEncoder  # noqa: E402


POLICY_NAMESPACE = uuid.UUID("e1e59f9e-a77f-4a96-bb66-1e2f694cb2dc")


@dataclass(frozen=True)
class PolicySeed:
    slug: str
    name: str
    status: str
    policy_type: str
    priority: int
    op: str
    target: str
    params_anchor: Optional[str]
    risk_anchor: Optional[str]
    thresholds: tuple[float, float, float, float]  # action, resource, data, risk
    scoring_mode: str
    weights: Optional[tuple[float, float, float, float]]
    drift_threshold: Optional[float]
    modification_spec: Optional[dict]
    notes: Optional[str]


def build_policy_id(tenant_id: str, slug: str) -> str:
    """Generate deterministic UUIDv5 policy IDs from tenant + slug."""
    return str(uuid.uuid5(POLICY_NAMESPACE, f"{tenant_id}:{slug}"))


def seeds() -> list[PolicySeed]:
    return [
        PolicySeed(
            slug="allow-production-credential-access-authorized-pipelines-only",
            name="Allow Production Credential Access - Authorized Pipelines Only",
            status="active",
            policy_type="context_allow",
            priority=0,
            op="read database credentials from secrets store",
            target="production secrets manager",
            params_anchor=None,
            risk_anchor="agent has explicit authorization for credential rotation or deployment pipeline",
            thresholds=(0.33, 0.16, 0.44, 0.15),
            scoring_mode="weighted-avg",
            weights=(1.0, 1.0, 1.6, 1.0),
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed only when session context confirms deployment or rotation authorization.",
        ),
        PolicySeed(
            slug="allow-pii-query-approved-analytics-access",
            name="Allow PII Query - Approved Analytics Access",
            status="active",
            policy_type="context_allow",
            priority=10,
            op="query user records from database",
            target="postgres users table",
            params_anchor='{"columns": ["email", "phone"], "operation": "SELECT"}',
            risk_anchor='{"data_access_approved": true, "purpose": "analytics"}',
            thresholds=(0.8, 0.8, 0.8, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed when analyst has active data access approval for analytics.",
        ),
        PolicySeed(
            slug="allow-financial-report-generation",
            name="Allow Financial Report Generation",
            status="disabled",
            policy_type="context_allow",
            priority=20,
            op="generate summary report",
            target="financial transactions database",
            params_anchor=None,
            risk_anchor="user explicitly requested monthly financial report",
            thresholds=(0.78, 0.78, 0.78, 0.78),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed only when session context confirms financial report intent.",
        ),
        PolicySeed(
            slug="allow-bulk-export-approved-etl-pipeline",
            name="Allow Bulk Export - Approved ETL Pipeline",
            status="disabled",
            policy_type="context_allow",
            priority=30,
            op="export all records",
            target="customer database",
            params_anchor=None,
            risk_anchor="export is part of an approved ETL pipeline or authorized data migration task",
            thresholds=(0.8, 0.8, 0.8, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=0.35,
            modification_spec=None,
            notes="Denied by default. Allowed when export is confirmed as part of a known approved pipeline.",
        ),
        PolicySeed(
            slug="allow-web-search-authorized-research-tasks-disabled",
            name="[DISABLED] Allow Web Search - Authorized Research Tasks",
            status="disabled",
            policy_type="context_allow",
            priority=5,
            op="search for information on the web",
            target="google.com",
            params_anchor=None,
            risk_anchor="agent has been granted web access for an authorized research task",
            thresholds=(0.85, 0.85, 0.85, 0.85),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Disabled. Would allow web search only when agent has explicit research authorization.",
        ),
        PolicySeed(
            slug="allow-threat-actor-attribution-export-security-team-authorized",
            name="Allow Threat Actor Attribution Export - Security Team Authorized",
            status="disabled",
            policy_type="context_allow",
            priority=0,
            op="export threat actor attribution records",
            target="threat intelligence database",
            params_anchor='{"export_type": "bulk", "classification": ["nation-state", "APT"]}',
            risk_anchor="request is authorized by security team for threat reporting or intelligence sharing",
            thresholds=(0.8, 0.8, 0.78, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed when security team has explicitly authorized the export.",
        ),
        PolicySeed(
            slug="allow-ioc-feed-query-active-incident-confirmed",
            name="Allow IOC Feed Query - Active Incident Confirmed",
            status="active",
            policy_type="context_allow",
            priority=10,
            op="query indicators of compromise from threat feed",
            target="IOC threat intelligence feed",
            params_anchor='{"indicator_types": ["ip", "domain", "hash"]}',
            risk_anchor='{"active_incident": true}',
            thresholds=(0.8, 0.8, 0.78, 0.78),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed when session context confirms an active incident is open.",
        ),
        PolicySeed(
            slug="allow-threat-intel-enrichment-during-active-incident",
            name="Allow Threat Intel Enrichment During Active Incident",
            status="disabled",
            policy_type="context_allow",
            priority=20,
            op="enrich threat indicators from intelligence platform",
            target="MISP or threat intel platform",
            params_anchor=None,
            risk_anchor="analyst is actively investigating a security incident",
            thresholds=(0.78, 0.78, 0.78, 0.78),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed when session context confirms an open incident investigation.",
        ),
        PolicySeed(
            slug="allow-osint-collection-authorized-threat-hunt",
            name="Allow OSINT Collection - Authorized Threat Hunt",
            status="active",
            policy_type="context_allow",
            priority=40,
            op="collect open source intelligence on target",
            target="public internet or OSINT sources",
            params_anchor=None,
            risk_anchor="OSINT collection is scoped to an authorized target as part of an active threat hunt",
            thresholds=(0.8, 0.78, 0.8, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=0.4,
            modification_spec=None,
            notes="Denied by default. Allowed when OSINT scope is confirmed as part of an authorized threat hunt.",
        ),
        PolicySeed(
            slug="block-file-encryption-on-production-file-systems",
            name="Block File Encryption on Production File Systems",
            status="disabled",
            policy_type="forbidden",
            priority=0,
            op="encrypt or overwrite files on disk",
            target="production file system or shared network drive",
            params_anchor="bulk file operation across multiple directories",
            risk_anchor=None,
            thresholds=(0.78, 0.78, 0.78, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Hard block - any agent-initiated bulk file encryption is ransomware-class behavior.",
        ),
        PolicySeed(
            slug="allow-bulk-file-enumeration-authorized-forensic-scan",
            name="Allow Bulk File Enumeration - Authorized Forensic Scan",
            status="active",
            policy_type="context_allow",
            priority=10,
            op="enumerate and read files across directories",
            target="file system",
            params_anchor='{"scope": "recursive", "targets": ["extensions", "metadata"]}',
            risk_anchor='{"authorized_scan": true, "purpose": "forensic_investigation"}',
            thresholds=(0.8, 0.8, 0.78, 0.78),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Denied by default. Allowed when scan is authorized for endpoint detection or forensic investigation.",
        ),
        PolicySeed(
            slug="deny-shadow-copy-or-backup-deletion",
            name="Deny Shadow Copy or Backup Deletion",
            status="disabled",
            policy_type="forbidden",
            priority=1,
            op="delete or disable volume shadow copies or backups",
            target="Windows VSS or backup storage",
            params_anchor=None,
            risk_anchor=None,
            thresholds=(0.78, 0.78, 0.8, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=None,
            modification_spec=None,
            notes="Shadow copy deletion is a canonical ransomware pre-encryption step. Always block.",
        ),
        PolicySeed(
            slug="allow-lateral-movement-scan-pentest-or-incident-response",
            name="Allow Lateral Movement Scan - Pentest or Incident Response",
            status="disabled",
            policy_type="context_allow",
            priority=30,
            op="connect to or scan remote hosts on internal network",
            target="internal network segment or subnet",
            params_anchor='{"protocol": "SMB", "scope": "subnet_scan"}',
            risk_anchor="network scan is authorized as part of a penetration test or active incident response",
            thresholds=(0.8, 0.8, 0.8, 0.8),
            scoring_mode="min",
            weights=None,
            drift_threshold=0.3,
            modification_spec=None,
            notes="Denied by default. Allowed when scan is authorized for pentest or incident response.",
        ),
    ]


def to_boundary(seed: PolicySeed, tenant_id: str, now_ts: float) -> DesignBoundary:
    policy_id = build_policy_id(tenant_id, seed.slug)
    weight_model = None
    if seed.weights is not None:
        weight_model = SliceWeights(
            action=seed.weights[0],
            resource=seed.weights[1],
            data=seed.weights[2],
            risk=seed.weights[3],
        )

    return DesignBoundary(
        id=policy_id,
        name=seed.name,
        tenant_id=tenant_id,
        status=seed.status,
        policy_type=seed.policy_type,
        priority=seed.priority,
        match=PolicyMatch(op=seed.op, t=seed.target, p=seed.params_anchor, ctx=seed.risk_anchor),
        thresholds=SliceThresholds(
            action=seed.thresholds[0],
            resource=seed.thresholds[1],
            data=seed.thresholds[2],
            risk=seed.thresholds[3],
        ),
        scoring_mode=seed.scoring_mode,
        weights=weight_model,
        drift_threshold=seed.drift_threshold,
        modification_spec=seed.modification_spec,
        notes=seed.notes,
        created_at=now_ts,
        updated_at=now_ts,
    )


def seed_policies(tenant_id: str) -> None:
    encoder = PolicyEncoder()
    created = 0
    updated = 0

    for s in seeds():
        now_ts = time.time()
        boundary = to_boundary(s, tenant_id, now_ts)
        existing = fetch_policy_record(tenant_id, boundary.id)
        if existing:
            boundary = boundary.model_copy(update={"created_at": existing.created_at, "updated_at": now_ts})
            update_policy_record(boundary, tenant_id)
            updated += 1
            verb = "updated"
        else:
            create_policy_record(boundary, tenant_id)
            created += 1
            verb = "created"

        rule_vector = encoder.encode(boundary)
        payload = {
            "boundary": boundary.model_dump(),
            "anchors": build_anchor_payload(rule_vector),
        }
        metadata = {
            "policy_id": boundary.id,
            "boundary_name": boundary.name,
            "status": boundary.status,
            "policy_type": boundary.policy_type,
        }
        upsert_policy_payload(tenant_id, boundary.id, payload, metadata)

        print(f"[{verb}] {boundary.id} :: {boundary.name}")

    print("\nDone.")
    print(f"Tenant: {tenant_id}")
    print(f"Policies created: {created}")
    print(f"Policies updated: {updated}")
    print(f"Total seeded: {len(seeds())}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Seed v2-compatible demo policies")
    parser.add_argument(
        "--tenant-id",
        default="demo-tenant",
        help="Tenant ID to seed policies into (default: demo-tenant)",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    seed_policies(args.tenant_id)


if __name__ == "__main__":
    main()
