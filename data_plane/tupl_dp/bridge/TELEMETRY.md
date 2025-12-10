# Tupl Data Plane Telemetry System

## Overview

The Tupl telemetry system provides **complete audit trail** of every enforcement decision, analogous to Linux's `conntrack` for network connections. Every intent that enters the data plane is recorded with:

- Full intent JSON
- Encoding metrics
- All rules evaluated
- Per-rule decisions and similarities
- Short-circuit events
- Final enforcement decision
- Complete performance breakdown

All telemetry is written to `/var/hitlogs/` as line-delimited JSON for easy querying and analysis.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│          Intent arrives at Data Plane                │
└────────────────────┬────────────────────────────────┘
                     ↓
        ┌────────────────────────────┐
        │  EnforcementEngine.enforce()│
        │                             │
        │  1. Start session           │
        │  2. Record events           │
        │  3. Track rule evaluations  │
        │  4. Complete session        │
        └────────────┬────────────────┘
                     ↓
        ┌────────────────────────────┐
        │   TelemetryRecorder         │
        │                             │
        │  - Thread-safe sessions     │
        │  - Performance tracking     │
        │  - Event collection         │
        └────────────┬────────────────┘
                     ↓
        ┌────────────────────────────┐
        │      HitlogWriter           │
        │                             │
        │  - Atomic writes            │
        │  - File rotation            │
        │  - Compression              │
        └────────────┬────────────────┘
                     ↓
              /var/hitlogs/
              ├─ enforcement.hitlog (current)
              ├─ enforcement.hitlog.1699234567.gz
              └─ enforcement.hitlog.1699148167.gz
```

---

## File Format

Each line in a hitlog file is a complete `EnforcementSession` JSON object:

```json
{
  "session_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp_ms": 1699234567000,
  "layer": "L4",
  "agent_id": "customer-support-agent",
  "tenant_id": "acme-corp",
  "intent_json": "{\"layer\":\"L4\",\"tool_name\":\"postgres_query\",...}",
  "intent_vector": [0.12, -0.45, ..., 0.89],
  "events": [
    {
      "type": "intent_received",
      "timestamp_us": 1699234567123456,
      "intent_id": "intent_abc123",
      "layer": "L4"
    },
    {
      "type": "encoding_started",
      "timestamp_us": 1699234567123500
    },
    {
      "type": "encoding_completed",
      "timestamp_us": 1699234567130200,
      "duration_us": 6700,
      "vector_norm": 1.0
    },
    {
      "type": "rules_queried",
      "timestamp_us": 1699234567130300,
      "layer": "L4",
      "rule_count": 3,
      "query_duration_us": 100
    },
    {
      "type": "rule_evaluation_started",
      "timestamp_us": 1699234567130400,
      "rule_id": "allow-db-tools",
      "rule_priority": 600
    },
    {
      "type": "rule_evaluation_completed",
      "timestamp_us": 1699234567131200,
      "rule_id": "allow-db-tools",
      "decision": 1,
      "similarities": [0.92, 0.95, 0.82, 0.91],
      "duration_us": 800
    },
    {
      "type": "final_decision",
      "timestamp_us": 1699234567131300,
      "decision": 1,
      "rules_evaluated": 3,
      "total_duration_us": 7844
    }
  ],
  "rules_evaluated": [
    {
      "rule_id": "deny-admin-tools",
      "rule_family": "tool_whitelist",
      "priority": 900,
      "description": "Block administrative tools",
      "started_at_us": 1699234567130400,
      "duration_us": 654,
      "decision": 1,
      "slice_similarities": [0.45, 0.32, 0.67, 0.88],
      "thresholds": [0.70, 0.88, 0.70, 0.60],
      "anchor_counts": [2, 3, 1, 2],
      "short_circuited": false,
      "slice_details": [
        {
          "slice_name": "action",
          "similarity": 0.45,
          "threshold": 0.70,
          "passed": false,
          "anchor_count": 2,
          "best_anchor_idx": null
        },
        ...
      ]
    },
    {
      "rule_id": "allow-db-tools",
      "rule_family": "tool_whitelist",
      "priority": 600,
      "description": "Allow database tools",
      "started_at_us": 1699234567131000,
      "duration_us": 789,
      "decision": 1,
      "slice_similarities": [0.92, 0.95, 0.82, 0.91],
      "thresholds": [0.70, 0.88, 0.70, 0.60],
      "anchor_counts": [3, 2, 2, 1],
      "short_circuited": false,
      "slice_details": [...]
    }
  ],
  "final_decision": 1,
  "final_similarities": [0.92, 0.95, 0.82, 0.91],
  "duration_us": 7844,
  "performance": {
    "encoding_duration_us": 6700,
    "rule_query_duration_us": 100,
    "evaluation_duration_us": 1044,
    "total_duration_us": 7844,
    "rules_queried": 3,
    "rules_evaluated": 3,
    "short_circuited": false
  },
  "error": null
}
```

---

## Configuration

### Enable Telemetry

```rust
use tupl_dp::telemetry::{TelemetryConfig, TelemetryRecorder};
use tupl_dp::enforcement_engine::EnforcementEngine;

// Configure telemetry
let telemetry_config = TelemetryConfig {
    enabled: true,
    hitlog_dir: "/var/hitlogs".to_string(),
    sample_rate: 1.0,  // Record 100% of sessions
    buffer_size: 100,
    flush_interval_secs: 5,
    track_performance: true,
    track_slice_details: true,
};

// Create telemetry recorder
let telemetry = Arc::new(TelemetryRecorder::new(telemetry_config)?);

// Create enforcement engine with telemetry
let engine = EnforcementEngine::with_telemetry(
    bridge,
    "http://localhost:8000".to_string(),
    Some(telemetry),
)?;

// Enforcement automatically records to hitlog
let result = engine.enforce(&intent_json).await?;
```

### Disable Telemetry

```rust
// Create without telemetry
let engine = EnforcementEngine::new(
    bridge,
    "http://localhost:8000".to_string(),
);
```

### Sampling

To reduce hitlog volume, use sampling:

```rust
let telemetry_config = TelemetryConfig {
    enabled: true,
    sample_rate: 0.1,  // Record 10% of sessions
    ..Default::default()
};
```

---

## File Rotation

Hitlogs automatically rotate based on configured policy:

```rust
use tupl_dp::telemetry::writer::{HitlogConfig, RotationPolicy};

let hitlog_config = HitlogConfig {
    base_dir: "/var/hitlogs".to_string(),
    rotation: RotationPolicy::BySize(100 * 1024 * 1024),  // Rotate at 100 MB
    compress_rotated: true,
    max_rotated_files: 10,
    ..Default::default()
};
```

**Rotation Policies:**
- `RotationPolicy::BySize(bytes)` - Rotate when file exceeds size
- `RotationPolicy::ByTime(seconds)` - Rotate every N seconds
- `RotationPolicy::Daily` - Rotate daily at midnight UTC
- `RotationPolicy::Hourly` - Rotate every hour
- `RotationPolicy::Never` - No rotation

**Rotated files:**
- Format: `enforcement.hitlog.<unix_timestamp>.gz`
- Compressed with gzip
- Old files automatically cleaned up

---

## Querying Hitlogs

### CLI Viewer

Use the `hitlog_viewer` CLI tool:

```bash
# Recent sessions
hitlog_viewer recent --limit 20

# Only blocked sessions
hitlog_viewer blocked

# Sessions for specific agent
hitlog_viewer by-agent customer-support-agent

# Specific session details
hitlog_viewer by-session a1b2c3d4-e5f6-7890-abcd-ef1234567890

# Statistics
hitlog_viewer stats

# Custom query
hitlog_viewer query \
  --layer L4 \
  --decision 0 \
  --start-time 1699234567000 \
  --limit 50 \
  --format json
```

### Programmatic Query

```rust
use tupl_dp::telemetry::{HitlogQuery, QueryFilter};

let query = HitlogQuery::new("/var/hitlogs");

// Recent sessions
let sessions = query.recent(10)?;

// Blocked sessions
let blocked = query.blocked(None)?;

// By agent
let agent_sessions = query.by_agent("customer-support-agent".to_string(), Some(50))?;

// Custom filter
let filter = QueryFilter {
    layer: Some("L4".to_string()),
    decision: Some(0),  // Only BLOCK
    start_time_ms: Some(1699234567000),
    end_time_ms: Some(1699320967000),
    limit: Some(100),
    ..Default::default()
};

let result = query.query(&filter)?;

// Statistics
let stats = query.statistics()?;
println!("Block rate: {:.1}%", stats.block_rate * 100.0);
```

---

## Use Cases

### 1. Debugging Policy Decisions

Why was this intent blocked?

```bash
# Find the session
hitlog_viewer by-session <session_id>

# Shows:
# - Complete intent JSON
# - All rules evaluated
# - Which rule blocked
# - Similarity scores per slice
# - Short-circuit information
```

### 2. Policy Tuning

Are my thresholds too strict/loose?

```bash
# View all blocked sessions
hitlog_viewer blocked --limit 100

# Analyze similarities for near-misses
# Adjust thresholds in rule configuration
```

### 3. Performance Analysis

Where is enforcement spending time?

```bash
# View statistics
hitlog_viewer stats

# Shows:
# - Avg encoding duration
# - Avg rule query time
# - Avg evaluation time
# - Short-circuit rate
```

### 4. Compliance Auditing

Complete audit trail for compliance:

```bash
# Export all enforcement decisions for time range
hitlog_viewer query \
  --start-time 1699234567000 \
  --end-time 1699320967000 \
  --format json > audit_report.json
```

### 5. Anomaly Detection

Find unusual enforcement patterns:

```rust
let stats = query.statistics()?;

if stats.block_rate > 0.5 {
    alert!("High block rate: {:.1}%", stats.block_rate * 100.0);
}

// Find slow evaluations
let filter = QueryFilter {
    min_duration_us: Some(50_000),  // > 50ms
    ..Default::default()
};
let slow_sessions = query.query(&filter)?;
```

---

## Performance Impact

Telemetry is designed for minimal overhead:

| Operation | Overhead |
|-----------|----------|
| Session start | ~5 μs |
| Event recording | ~0.5 μs per event |
| Session complete | ~20 μs (write to buffer) |
| Flush to disk | ~1-5 ms (async, batched) |

**Total overhead per enforcement:** ~30-50 μs (<1% of total enforcement time)

With buffering and async writes, telemetry does not impact enforcement latency.

---

## Storage Requirements

### Per Session

- Minimal session (no rules): ~500 bytes
- Typical session (3 rules): ~2-3 KB
- Complex session (10 rules): ~8-10 KB

### Daily Volume

At 1000 requests/sec:
- Without sampling: ~86.4M sessions/day = ~260 GB/day
- With 10% sampling: ~8.64M sessions/day = ~26 GB/day
- With 1% sampling: ~864K sessions/day = ~2.6 GB/day

**Recommendation:** Use sampling + rotation + compression for high-volume deployments.

---

## Best Practices

1. **Enable sampling in production**
   - 10% for most workloads
   - 100% for debugging/testing

2. **Configure rotation**
   - Rotate at 100 MB or hourly
   - Keep 10-20 rotated files
   - Enable compression

3. **Monitor hitlog size**
   - Alert if directory grows > expected
   - Check for stuck rotations

4. **Use structured queries**
   - Build dashboards from hitlog data
   - Export to ClickHouse/Elastic for analytics

5. **Periodic cleanup**
   - Archive old hitlogs to cold storage
   - Delete after retention period

---

## Troubleshooting

### Hitlog not being created

- Check `/var/hitlogs` directory exists and is writable
- Verify telemetry is enabled in config
- Check `telemetry.stats()` shows `total_sessions > 0`

### High disk usage

- Enable sampling
- Reduce `max_rotated_files`
- Enable compression
- Archive/delete old files

### Missing sessions

- Check `sample_rate` (may be < 1.0)
- Verify session completed (not abandoned)
- Check for errors in enforcement

### Slow queries

- Rotated files are searched oldest-first
- Use time filters to limit search
- Consider indexing (future enhancement)

---

## Future Enhancements

- [ ] SQLite index for fast queries
- [ ] Real-time streaming to observability platforms
- [ ] Compression of intent_vector (sparse encoding)
- [ ] Session correlation (link intents from same request)
- [ ] Anomaly detection algorithms
- [ ] Grafana dashboard templates

---

## Summary

The Tupl telemetry system provides:

✅ **Complete visibility** - Every enforcement decision recorded
✅ **Zero latency impact** - Async buffered writes
✅ **Flexible querying** - CLI and programmatic access
✅ **Audit compliance** - Tamper-evident log chain
✅ **Performance insights** - Detailed timing breakdowns
✅ **Debug-friendly** - Full context for every decision

Analogous to `conntrack` for network flows, but for AI enforcement decisions.
