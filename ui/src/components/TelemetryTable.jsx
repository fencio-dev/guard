import { useState, useEffect, useCallback } from 'react';
import { fetchSessions, fetchSessionDetail } from '../api/telemetry';

function formatTime(ms) {
  if (ms == null) return '—';
  return new Date(ms).toLocaleString();
}

function formatTimeShort(ms) {
  if (ms == null) return '—';
  const d = new Date(ms);
  return d.toLocaleTimeString();
}

function truncate(str, len = 12) {
  if (!str) return '—';
  return str.length > len ? str.slice(0, len) + '...' : str;
}

const DECISION_COLORS = {
  ALLOW:   { background: '#d4edda', color: '#155724' },
  DENY:    { background: '#f8d7da', color: '#721c24' },
  MODIFY:  { background: '#cce5ff', color: '#004085' },
  STEP_UP: { background: '#fff3cd', color: '#856404' },
  DEFER:   { background: '#e2e3e5', color: '#383d41' },
};

function decisionBadgeStyle(decision) {
  const colors = DECISION_COLORS[decision] ?? { background: '#e2e3e5', color: '#383d41' };
  return { ...styles.badge, ...colors };
}

function formatDuration(us) {
  if (us == null) return '—';
  return (us / 1000).toFixed(2) + ' ms';
}

const styles = {
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: 16,
  },
  heading: {
    fontSize: 15,
    fontWeight: 600,
    color: '#1a1a1a',
    display: 'flex',
    alignItems: 'center',
    gap: 10,
  },
  totalCount: {
    fontSize: 13,
    color: '#888',
    fontWeight: 400,
  },
  statusArea: {
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    fontSize: 13,
    color: '#555',
  },
  dot: (color) => ({
    display: 'inline-block',
    width: 8,
    height: 8,
    borderRadius: '50%',
    background: color,
    flexShrink: 0,
  }),
  offlineBanner: {
    background: '#fff3cd',
    border: '1px solid #ffc107',
    borderRadius: 4,
    padding: '10px 14px',
    fontSize: 13,
    color: '#856404',
    marginBottom: 16,
    width: '100%',
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
    fontSize: 12,
    fontFamily: 'monospace',
  },
  badge: {
    display: 'inline-block',
    fontSize: 11,
    fontWeight: 700,
    padding: '2px 8px',
    borderRadius: 4,
    letterSpacing: 0.5,
  },
  emptyState: {
    textAlign: 'center',
    color: '#888',
    fontSize: 13,
    fontStyle: 'italic',
    padding: '24px 0',
  },
  detailPanel: {
    borderTop: '2px solid #e8e8e8',
    marginTop: 28,
    paddingTop: 24,
  },
  detailHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: 16,
  },
  detailTitle: {
    fontSize: 14,
    fontWeight: 600,
    color: '#333',
  },
  closeButton: {
    background: 'none',
    border: 'none',
    fontSize: 18,
    cursor: 'pointer',
    color: '#888',
    lineHeight: 1,
    padding: '0 4px',
  },
  kvGrid: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: 20,
    marginBottom: 20,
  },
  kvItem: {
    display: 'flex',
    flexDirection: 'column',
    gap: 2,
  },
  kvLabel: {
    fontSize: 11,
    fontWeight: 600,
    color: '#888',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  kvValue: {
    fontSize: 13,
    color: '#1a1a1a',
    fontFamily: 'monospace',
  },
  pre: {
    background: '#f5f5f5',
    border: '1px solid #ddd',
    borderRadius: 4,
    padding: 14,
    fontSize: 12,
    fontFamily: 'monospace',
    overflowX: 'auto',
    whiteSpace: 'pre-wrap',
    wordBreak: 'break-all',
    color: '#1a1a1a',
  },
};

function SessionDetail({ session, onClose }) {
  const decision = session?.final_decision ?? '—';
  const badge = decisionBadgeStyle(decision);

  return (
    <div style={styles.detailPanel}>
      <div style={styles.detailHeader}>
        <span style={styles.detailTitle}>Session Detail</span>
        <button style={styles.closeButton} onClick={onClose} aria-label="Close">×</button>
      </div>
      <div style={styles.kvGrid}>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Session ID</span>
          <span style={styles.kvValue}>{session?.session_id ?? '—'}</span>
        </div>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Agent ID</span>
          <span style={styles.kvValue}>{session?.agent_id ?? '—'}</span>
        </div>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Tenant ID</span>
          <span style={styles.kvValue}>{session?.tenant_id ?? '—'}</span>
        </div>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Layer</span>
          <span style={styles.kvValue}>{session?.layer ?? '—'}</span>
        </div>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Decision</span>
          <span style={badge}>{decision}</span>
        </div>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Duration</span>
          <span style={styles.kvValue}>{formatDuration(session?.duration_us)}</span>
        </div>
        <div style={styles.kvItem}>
          <span style={styles.kvLabel}>Rules Evaluated</span>
          <span style={styles.kvValue}>{session?.rules_evaluated_count ?? '—'}</span>
        </div>
      </div>
      <pre style={styles.pre}>{JSON.stringify(session, null, 2)}</pre>
    </div>
  );
}

export default function TelemetryTable() {
  const [sessions, setSessions] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [offline, setOffline] = useState(false);
  const [totalCount, setTotalCount] = useState(0);
  const [lastUpdated, setLastUpdated] = useState(null);
  const [selectedSession, setSelectedSession] = useState(null);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [selectedRowId, setSelectedRowId] = useState(null);

  const poll = useCallback(async () => {
    try {
      const data = await fetchSessions({ limit: 50, offset: 0 });
      setSessions(data?.sessions ?? []);
      setTotalCount(data?.total_count ?? 0);
      setOffline(false);
      setError(null);
      setLastUpdated(new Date());
    } catch (err) {
      // TypeError = network/fetch failure; HTTP 5xx = backend error — both treated as offline
      const isNetworkError = err instanceof TypeError;
      const isServerError = err.message && /HTTP 5\d\d/.test(err.message);
      if (isNetworkError || isServerError) {
        setOffline(true);
      } else {
        setError(err.message ?? 'Failed to fetch sessions');
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    poll();
    const id = setInterval(poll, 5000);
    return () => clearInterval(id);
  }, [poll]);

  async function handleRowClick(session) {
    setSelectedRowId(session.session_id);
    setLoadingDetail(true);
    try {
      const detail = await fetchSessionDetail(session.session_id);
      // Merge: full action_history from detail + pre-computed fields (final_decision) from list row
      setSelectedSession({ ...session, ...(detail?.session ?? detail) });
    } catch (err) {
      setSelectedSession(session); // fall back to summary data
    } finally {
      setLoadingDetail(false);
    }
  }

  return (
    <div>
      <div style={styles.header}>
        <div style={styles.heading}>
          Telemetry
          <span style={styles.totalCount}>{totalCount} sessions</span>
        </div>
        <div style={styles.statusArea}>
          {offline ? (
            <>
              <span style={styles.dot('red')} />
              Guard not running
            </>
          ) : loading ? (
            'Loading...'
          ) : (
            <>
              <span style={styles.dot('green')} />
              Live
              {lastUpdated && (
                <span style={{ color: '#aaa' }}>· last updated {formatTimeShort(lastUpdated)}</span>
              )}
            </>
          )}
        </div>
      </div>

      {offline && (
        <div style={styles.offlineBanner}>
          ⚠ Guard backend is not reachable. Retrying...
        </div>
      )}

      {error && !offline && (
        <div style={{ ...styles.offlineBanner, background: '#f8d7da', borderColor: '#f5c6cb', color: '#721c24' }}>
          Error: {error}
        </div>
      )}

      <div style={styles.tableWrapper}>
        <table style={styles.table}>
          <thead>
            <tr>
              <th style={styles.th}>Time</th>
              <th style={styles.th}>Intent</th>
              <th style={styles.th}>Agent ID</th>
              <th style={styles.th}>Tenant ID</th>
              <th style={styles.th}>Layer</th>
              <th style={styles.th}>Decision</th>
              <th style={styles.th}>Duration</th>
              <th style={styles.th}>Rules</th>
            </tr>
          </thead>
          <tbody>
            {sessions.length === 0 ? (
              <tr>
                <td colSpan={8} style={styles.emptyState}>
                  No sessions recorded yet.
                </td>
              </tr>
            ) : (
              sessions.map((s) => {
                const isSelected = s.session_id === selectedRowId;
                const decision = s.final_decision ?? '—';
                const badge = decisionBadgeStyle(decision);
                const rowBg = isSelected ? '#e8f0fe' : undefined;

                return (
                  <tr
                    key={s.session_id}
                    onClick={() => handleRowClick(s)}
                    style={{ cursor: 'pointer', background: rowBg }}
                    onMouseEnter={(e) => {
                      if (!isSelected) e.currentTarget.style.background = '#f5f5f5';
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = isSelected ? '#e8f0fe' : '';
                    }}
                  >
                    <td style={styles.td}>{formatTime(s.timestamp_ms)}</td>
                    <td style={styles.td}>{s.intent_summary ?? '—'}</td>
                    <td style={styles.td}>{truncate(s.agent_id)}</td>
                    <td style={styles.td}>{truncate(s.tenant_id)}</td>
                    <td style={styles.td}>{s.layer ?? '—'}</td>
                    <td style={styles.td}>
                      <span style={badge}>{decision}</span>
                    </td>
                    <td style={styles.td}>{formatDuration(s.duration_us)}</td>
                    <td style={styles.td}>{s.rules_evaluated_count ?? '—'}</td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {loadingDetail && (
        <div style={{ marginTop: 16, fontSize: 13, color: '#888' }}>Loading session detail...</div>
      )}

      {selectedSession && !loadingDetail && (
        <SessionDetail session={selectedSession} onClose={() => { setSelectedSession(null); setSelectedRowId(null); }} />
      )}
    </div>
  );
}
