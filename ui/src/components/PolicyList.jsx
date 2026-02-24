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
          onSuccess={() => { setShowForm(false); load(); }}
          onCancel={() => setShowForm(false)}
        />
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
                  <td style={styles.td}>{formatDate(policy.createdAt)}</td>
                  <td style={styles.td}>
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
