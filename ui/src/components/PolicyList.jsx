import { useState, useEffect } from 'react';
import { fetchPolicies, deletePolicy } from '../api/policies';
import PolicyForm from './PolicyForm';

function formatDate(timestamp) {
  return new Date(timestamp * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

const styles = {
  toolbar: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: 16,
  },
  count: {
    fontSize: 14,
    color: '#555',
  },
  addButton: {
    fontSize: 13,
    padding: '6px 14px',
    border: '1px solid #1a1a1a',
    borderRadius: 4,
    background: '#1a1a1a',
    color: '#fff',
    cursor: 'pointer',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: 14,
  },
  th: {
    textAlign: 'left',
    padding: '8px 12px',
    borderBottom: '2px solid #ddd',
    fontWeight: 600,
    color: '#333',
    whiteSpace: 'nowrap',
  },
  td: {
    padding: '8px 12px',
    borderBottom: '1px solid #eee',
    verticalAlign: 'middle',
  },
  statusActive: {
    color: '#2d8a4e',
    fontWeight: 500,
  },
  statusDisabled: {
    color: '#999',
  },
  effectAllow: {
    color: '#2d8a4e',
    fontWeight: 500,
  },
  effectDeny: {
    color: '#c0392b',
    fontWeight: 500,
  },
  deleteButton: {
    fontSize: 12,
    padding: '4px 10px',
    border: '1px solid #e0a0a0',
    borderRadius: 4,
    background: '#fdf2f2',
    color: '#c0392b',
    cursor: 'pointer',
  },
  deleteButtonDisabled: {
    fontSize: 12,
    padding: '4px 10px',
    border: '1px solid #ddd',
    borderRadius: 4,
    background: '#f5f5f5',
    color: '#aaa',
    cursor: 'not-allowed',
  },
  editButton: {
    fontSize: 12,
    padding: '4px 10px',
    border: '1px solid #b0c4de',
    borderRadius: 4,
    background: '#f0f4fa',
    color: '#2255a4',
    cursor: 'pointer',
    marginRight: 6,
  },
  viewButton: {
    fontSize: 12,
    padding: '4px 10px',
    border: '1px solid #c8c8c8',
    borderRadius: 4,
    background: '#f5f5f5',
    color: '#444',
    cursor: 'pointer',
    marginRight: 6,
  },
  modalOverlay: {
    position: 'fixed',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    background: 'rgba(0,0,0,0.35)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    zIndex: 1000,
  },
  modalBox: {
    background: '#fff',
    borderRadius: 6,
    padding: '28px 32px',
    maxWidth: 560,
    width: '100%',
    maxHeight: '80vh',
    overflowY: 'auto',
    boxShadow: '0 4px 24px rgba(0,0,0,0.15)',
  },
  modalTitle: {
    fontSize: 16,
    fontWeight: 600,
    marginBottom: 20,
    color: '#1a1a1a',
  },
  modalGrid: {
    display: 'grid',
    gridTemplateColumns: '160px 1fr',
    gap: '8px 16px',
    fontSize: 13,
  },
  modalLabel: {
    color: '#666',
    fontWeight: 500,
    paddingTop: 1,
  },
  modalValue: {
    color: '#1a1a1a',
    wordBreak: 'break-word',
  },
  modalDivider: {
    gridColumn: '1 / -1',
    borderTop: '1px solid #eee',
    margin: '8px 0',
  },
  modalClose: {
    marginTop: 24,
    fontSize: 13,
    padding: '6px 16px',
    border: '1px solid #c8c8c8',
    borderRadius: 4,
    background: '#f5f5f5',
    color: '#333',
    cursor: 'pointer',
  },
  empty: {
    textAlign: 'center',
    padding: '48px 0',
    color: '#888',
  },
  message: {
    color: '#888',
    fontSize: 14,
  },
  error: {
    color: '#c0392b',
    fontSize: 14,
  },
};

export default function PolicyList() {
  const [policies, setPolicies] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [deletingIds, setDeletingIds] = useState(new Set());
  const [showForm, setShowForm] = useState(false);
  const [selectedPolicy, setSelectedPolicy] = useState(null);
  const [viewPolicy, setViewPolicy] = useState(null);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchPolicies();
      setPolicies(data);
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleDelete(id) {
    setDeletingIds((prev) => new Set(prev).add(id));
    try {
      await deletePolicy(id);
      await load();
    } catch (err) {
      setError(err.message);
    } finally {
      setDeletingIds((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    }
  }

  return (
    <div>
      <div style={styles.toolbar}>
        <span style={styles.count}>
          {loading ? '' : `${policies.length} ${policies.length === 1 ? 'policy' : 'policies'}`}
        </span>
        <button style={styles.addButton} onClick={() => setShowForm(true)}>
          Add Policy
        </button>
      </div>

      {showForm && (
        <PolicyForm
          policy={selectedPolicy}
          onSuccess={() => { setShowForm(false); setSelectedPolicy(null); load(); }}
          onCancel={() => { setShowForm(false); setSelectedPolicy(null); }}
        />
      )}

      {viewPolicy && (
        <div style={styles.modalOverlay} onClick={() => setViewPolicy(null)}>
          <div style={styles.modalBox} onClick={(e) => e.stopPropagation()}>
            <div style={styles.modalTitle}>Policy Details</div>
            <div style={styles.modalGrid}>

              <span style={styles.modalLabel}>ID</span>
              <span style={styles.modalValue}>{viewPolicy.id}</span>

              <span style={styles.modalLabel}>Name</span>
              <span style={styles.modalValue}>{viewPolicy.name}</span>

              <span style={styles.modalLabel}>Tenant ID</span>
              <span style={styles.modalValue}>{viewPolicy.tenant_id ?? '—'}</span>

              <span style={styles.modalLabel}>Status</span>
              <span style={styles.modalValue}>{viewPolicy.status}</span>

              <span style={styles.modalLabel}>Policy Type</span>
              <span style={styles.modalValue}>{viewPolicy.policy_type ?? viewPolicy.type ?? '—'}</span>

              <span style={styles.modalLabel}>Priority</span>
              <span style={styles.modalValue}>{viewPolicy.priority ?? '—'}</span>

              <div style={styles.modalDivider} />

              <span style={styles.modalLabel}>Match: Operation</span>
              <span style={styles.modalValue}>{viewPolicy.match?.op ?? viewPolicy.match?.operation ?? '—'}</span>

              <span style={styles.modalLabel}>Match: Target/Tool</span>
              <span style={styles.modalValue}>{viewPolicy.match?.t ?? viewPolicy.match?.target_tool ?? viewPolicy.match?.tool ?? viewPolicy.match?.target ?? '—'}</span>

              {viewPolicy.match?.parameters && (
                <>
                  <span style={styles.modalLabel}>Match: Parameters</span>
                  <span style={styles.modalValue}>{JSON.stringify(viewPolicy.match.parameters)}</span>
                </>
              )}

              {viewPolicy.match?.risk_context && (
                <>
                  <span style={styles.modalLabel}>Match: Risk Context</span>
                  <span style={styles.modalValue}>{JSON.stringify(viewPolicy.match.risk_context)}</span>
                </>
              )}

              <div style={styles.modalDivider} />

              <span style={styles.modalLabel}>Threshold: Action</span>
              <span style={styles.modalValue}>{viewPolicy.thresholds?.action ?? '—'}</span>

              <span style={styles.modalLabel}>Threshold: Resource</span>
              <span style={styles.modalValue}>{viewPolicy.thresholds?.resource ?? '—'}</span>

              <span style={styles.modalLabel}>Threshold: Data</span>
              <span style={styles.modalValue}>{viewPolicy.thresholds?.data ?? '—'}</span>

              <span style={styles.modalLabel}>Threshold: Risk</span>
              <span style={styles.modalValue}>{viewPolicy.thresholds?.risk ?? '—'}</span>

              {viewPolicy.weights && (
                <>
                  <div style={styles.modalDivider} />
                  <span style={styles.modalLabel}>Weight: Action</span>
                  <span style={styles.modalValue}>{viewPolicy.weights.action ?? '—'}</span>

                  <span style={styles.modalLabel}>Weight: Resource</span>
                  <span style={styles.modalValue}>{viewPolicy.weights.resource ?? '—'}</span>

                  <span style={styles.modalLabel}>Weight: Data</span>
                  <span style={styles.modalValue}>{viewPolicy.weights.data ?? '—'}</span>

                  <span style={styles.modalLabel}>Weight: Risk</span>
                  <span style={styles.modalValue}>{viewPolicy.weights.risk ?? '—'}</span>
                </>
              )}

              {viewPolicy.drift_threshold != null && (
                <>
                  <div style={styles.modalDivider} />
                  <span style={styles.modalLabel}>Drift Threshold</span>
                  <span style={styles.modalValue}>{viewPolicy.drift_threshold}</span>
                </>
              )}

              {viewPolicy.notes && (
                <>
                  <div style={styles.modalDivider} />
                  <span style={styles.modalLabel}>Notes</span>
                  <span style={styles.modalValue}>{viewPolicy.notes}</span>
                </>
              )}

              <div style={styles.modalDivider} />

              <span style={styles.modalLabel}>Created At</span>
              <span style={styles.modalValue}>{formatDate(viewPolicy.created_at)}</span>

              <span style={styles.modalLabel}>Updated At</span>
              <span style={styles.modalValue}>{viewPolicy.updated_at ? formatDate(viewPolicy.updated_at) : '—'}</span>

            </div>
            <button style={styles.modalClose} onClick={() => setViewPolicy(null)}>Close</button>
          </div>
        </div>
      )}

      {loading && <p style={styles.message}>Loading...</p>}
      {error && <p style={styles.error}>{error}</p>}

      {!loading && !error && policies.length === 0 && (
        <div style={styles.empty}>No policies found.</div>
      )}

      {!loading && !error && policies.length > 0 && (
        <table style={styles.table}>
          <thead>
            <tr>
              <th style={styles.th}>Name</th>
              <th style={styles.th}>Status</th>
              <th style={styles.th}>Type</th>
              <th style={styles.th}>Layer</th>
              <th style={styles.th}>Effect</th>
              <th style={styles.th}>Created</th>
              <th style={styles.th}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {policies.map((policy) => {
              const isDeleting = deletingIds.has(policy.id);
              return (
                <tr key={policy.id}>
                  <td style={styles.td}>{policy.name}</td>
                  <td style={styles.td}>
                    <span
                      style={
                        policy.status === 'active'
                          ? styles.statusActive
                          : styles.statusDisabled
                      }
                    >
                      {policy.status}
                    </span>
                  </td>
                  <td style={styles.td}>{policy.type}</td>
                  <td style={styles.td}>{policy.layer ?? '—'}</td>
                  <td style={styles.td}>
                    <span
                      style={
                        policy.rules?.effect === 'allow'
                          ? styles.effectAllow
                          : styles.effectDeny
                      }
                    >
                      {policy.rules?.effect}
                    </span>
                  </td>
                  <td style={styles.td}>{formatDate(policy.created_at)}</td>
                  <td style={styles.td}>
                    <button
                      style={styles.viewButton}
                      onClick={() => setViewPolicy(policy)}
                    >
                      View
                    </button>
                    <button
                      style={styles.editButton}
                      disabled={isDeleting}
                      onClick={() => { setSelectedPolicy(policy); setShowForm(true); }}
                    >
                      Edit
                    </button>
                    <button
                      style={isDeleting ? styles.deleteButtonDisabled : styles.deleteButton}
                      disabled={isDeleting}
                      onClick={() => handleDelete(policy.id)}
                    >
                      {isDeleting ? 'Deleting...' : 'Delete'}
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
