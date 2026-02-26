const SLICE_LABELS = ['action', 'resource', 'data', 'risk'];

const BADGE_STYLES = {
  ALLOW:   { background: '#d4edda', color: '#155724' },
  DENY:    { background: '#f8d7da', color: '#721c24' },
  MODIFY:  { background: '#fff3cd', color: '#856404' },
  STEP_UP: { background: '#cce5ff', color: '#004085' },
  DEFER:   { background: '#e2e3e5', color: '#383d41' },
};

const styles = {
  panel: {
    borderTop: '2px solid #e8e8e8',
    marginTop: 28,
    paddingTop: 24,
  },
  panelTitle: {
    fontSize: 14,
    fontWeight: 600,
    color: '#333',
    marginBottom: 16,
  },
  badge: {
    display: 'inline-block',
    fontSize: 20,
    fontWeight: 700,
    padding: '8px 24px',
    borderRadius: 6,
    letterSpacing: 1,
    marginBottom: 16,
  },
  summaryRow: {
    display: 'flex',
    gap: 32,
    flexWrap: 'wrap',
    marginBottom: 20,
  },
  summaryItem: {
    display: 'flex',
    flexDirection: 'column',
    gap: 2,
  },
  summaryLabel: {
    fontSize: 11,
    fontWeight: 600,
    color: '#888',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  summaryValue: {
    fontSize: 13,
    color: '#1a1a1a',
    fontFamily: 'monospace',
  },
  tableWrapper: {
    overflowX: 'auto',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: 13,
  },
  th: {
    textAlign: 'left',
    padding: '8px 12px',
    background: '#f5f5f5',
    borderBottom: '1px solid #ddd',
    fontWeight: 600,
    color: '#555',
    fontSize: 12,
  },
  td: {
    padding: '8px 12px',
    borderBottom: '1px solid #eee',
    color: '#1a1a1a',
    fontFamily: 'monospace',
    fontSize: 12,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: 600,
    color: '#333',
    marginBottom: 10,
    marginTop: 20,
  },
  barLabelRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    marginBottom: 6,
  },
  barLabel: {
    fontSize: 12,
    color: '#555',
    fontFamily: 'monospace',
    width: 60,
    flexShrink: 0,
  },
  barContainer: {
    flex: 1,
    background: '#e8e8e8',
    borderRadius: 3,
    height: 10,
    overflow: 'hidden',
  },
  barValue: {
    fontSize: 12,
    color: '#1a1a1a',
    fontFamily: 'monospace',
    width: 36,
    textAlign: 'right',
    flexShrink: 0,
  },
  preBlock: {
    background: '#f5f5f5',
    border: '1px solid #ddd',
    borderRadius: 4,
    padding: '12px 16px',
    fontSize: 12,
    fontFamily: 'monospace',
    overflowX: 'auto',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-all',
    marginTop: 8,
  },
};

function DecisionBadge({ decision }) {
  const colors = BADGE_STYLES[decision] ?? { background: '#e2e3e5', color: '#383d41' };
  return (
    <div style={{ ...styles.badge, ...colors }}>
      {decision ?? '—'}
    </div>
  );
}

function SliceBars({ similarities }) {
  if (!Array.isArray(similarities)) return null;
  return (
    <div>
      {SLICE_LABELS.map((label, i) => {
        const value = similarities[i] ?? 0;
        const pct = Math.min(Math.max(value, 0), 1) * 100;
        return (
          <div key={label} style={styles.barLabelRow}>
            <span style={styles.barLabel}>{label}</span>
            <div style={styles.barContainer}>
              <div style={{ width: `${pct}%`, height: '100%', background: '#4a90d9', borderRadius: 3 }} />
            </div>
            <span style={styles.barValue}>{value.toFixed(2)}</span>
          </div>
        );
      })}
    </div>
  );
}

function EvidenceTable({ evidence }) {
  if (!Array.isArray(evidence) || evidence.length === 0) return null;
  return (
    <div style={styles.tableWrapper}>
      <table style={styles.table}>
        <thead>
          <tr>
            <th style={styles.th}>Policy Name</th>
            <th style={styles.th}>Effect</th>
            <th style={styles.th}>Match</th>
            <th style={styles.th}>Triggering Slice</th>
            <th style={styles.th}>Sims (action/resource/data/risk)</th>
          </tr>
        </thead>
        <tbody>
          {evidence.map((entry, i) => {
            const sims = Array.isArray(entry.similarities) ? entry.similarities : [];
            const simsFormatted = SLICE_LABELS.map((_, idx) =>
              sims[idx] != null ? sims[idx].toFixed(2) : '—'
            ).join(' / ');
            return (
              <tr key={i}>
                <td style={styles.td}>{entry.boundary_name || entry.boundary_id || '—'}</td>
                <td style={styles.td}>{entry.effect ?? '—'}</td>
                <td style={styles.td}>{entry.decision === 1 ? 'matched' : 'no match'}</td>
                <td style={styles.td}>{entry.triggering_slice ?? '—'}</td>
                <td style={styles.td}>{simsFormatted}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export default function EnforcementResultPanel({ result }) {
  if (!result) return null;

  const {
    decision,
    drift_score,
    drift_triggered,
    slice_similarities,
    evidence,
    modified_params,
  } = result;

  return (
    <div style={styles.panel}>
      <div style={styles.panelTitle}>Result</div>

      <DecisionBadge decision={decision} />

      <div style={styles.summaryRow}>
        <div style={styles.summaryItem}>
          <span style={styles.summaryLabel}>Drift Score</span>
          <span style={styles.summaryValue}>
            {drift_score != null ? drift_score.toFixed(4) : '—'}
          </span>
        </div>
        <div style={styles.summaryItem}>
          <span style={styles.summaryLabel}>Drift Triggered</span>
          <span style={styles.summaryValue}>{drift_triggered ? 'yes' : 'no'}</span>
        </div>
      </div>

      <div style={styles.sectionTitle}>Slice Similarities</div>
      <SliceBars similarities={slice_similarities} />

      <div style={styles.sectionTitle}>Evidence</div>
      <EvidenceTable evidence={evidence} />

      {modified_params != null && (
        <>
          <div style={styles.sectionTitle}>Modified Params</div>
          <pre style={styles.preBlock}>{JSON.stringify(modified_params, null, 2)}</pre>
        </>
      )}
    </div>
  );
}
