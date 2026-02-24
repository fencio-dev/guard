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
  badgeAllow: {
    display: 'inline-block',
    fontSize: 20,
    fontWeight: 700,
    padding: '8px 24px',
    borderRadius: 6,
    background: '#d4edda',
    color: '#155724',
    letterSpacing: 1,
    marginBottom: 16,
  },
  badgeBlock: {
    display: 'inline-block',
    fontSize: 20,
    fontWeight: 700,
    padding: '8px 24px',
    borderRadius: 6,
    background: '#f8d7da',
    color: '#721c24',
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
  noTrace: {
    fontSize: 13,
    color: '#888',
    fontStyle: 'italic',
    marginTop: 4,
  },
  traceTitle: {
    fontSize: 13,
    fontWeight: 600,
    color: '#333',
    marginBottom: 10,
  },
};

export default function EnforcementResultPanel({ result }) {
  if (!result) return null;

  const decision = result.decision;
  const badge = decision === 'ALLOW' ? styles.badgeAllow : styles.badgeBlock;

  const latency = result.enforcement_latency_ms?.toFixed(2) ?? '—';
  const requestId = result.metadata?.request_id ?? '—';
  const policyMatched = result.metadata?.policy_matched ?? '—';
  const trace = result.metadata?.canonicalization_trace ?? [];

  return (
    <div style={styles.panel}>
      <div style={styles.panelTitle}>Result</div>

      <div style={badge}>{decision ?? '—'}</div>

      <div style={styles.summaryRow}>
        <div style={styles.summaryItem}>
          <span style={styles.summaryLabel}>Latency</span>
          <span style={styles.summaryValue}>{latency} ms</span>
        </div>
        <div style={styles.summaryItem}>
          <span style={styles.summaryLabel}>Request ID</span>
          <span style={styles.summaryValue}>{requestId}</span>
        </div>
        <div style={styles.summaryItem}>
          <span style={styles.summaryLabel}>Policy Matched</span>
          <span style={styles.summaryValue}>{policyMatched}</span>
        </div>
      </div>

      <div style={styles.traceTitle}>Canonicalization Trace</div>
      {trace.length === 0 ? (
        <p style={styles.noTrace}>No canonicalization trace available.</p>
      ) : (
        <div style={styles.tableWrapper}>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>Field</th>
                <th style={styles.th}>Raw Input</th>
                <th style={styles.th}>Canonical</th>
                <th style={styles.th}>Confidence</th>
                <th style={styles.th}>Source</th>
              </tr>
            </thead>
            <tbody>
              {trace.map((entry, i) => (
                <tr key={i}>
                  <td style={styles.td}>{entry.field ?? '—'}</td>
                  <td style={styles.td}>{entry.raw_input ?? '—'}</td>
                  <td style={styles.td}>{entry.prediction?.canonical ?? '—'}</td>
                  <td style={styles.td}>
                    {entry.prediction?.confidence != null
                      ? `${Math.round(entry.prediction.confidence * 100)}%`
                      : '—'}
                  </td>
                  <td style={styles.td}>{entry.prediction?.source ?? '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
