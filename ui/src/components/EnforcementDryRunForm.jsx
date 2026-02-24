import { useState } from 'react';
import { runEnforce } from '../api/enforce';
import EnforcementResultPanel from './EnforcementResultPanel';

const SENSITIVITY_OPTIONS = ['public', 'internal', 'secret'];

const styles = {
  panel: {
    border: '1px solid #ddd',
    borderRadius: 6,
    padding: 24,
    marginBottom: 24,
    background: '#fafafa',
  },
  panelTitle: {
    fontSize: 15,
    fontWeight: 600,
    marginBottom: 20,
    color: '#1a1a1a',
  },
  fieldset: {
    border: '1px solid #e8e8e8',
    borderRadius: 4,
    padding: '16px 20px',
    marginBottom: 16,
  },
  legend: {
    fontSize: 13,
    fontWeight: 600,
    color: '#555',
    padding: '0 6px',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '14px 24px',
  },
  field: {
    display: 'flex',
    flexDirection: 'column',
    gap: 4,
  },
  label: {
    fontSize: 13,
    fontWeight: 500,
    color: '#333',
  },
  input: {
    fontSize: 13,
    padding: '6px 10px',
    border: '1px solid #ccc',
    borderRadius: 4,
    fontFamily: 'inherit',
    background: '#fff',
  },
  select: {
    fontSize: 13,
    padding: '6px 10px',
    border: '1px solid #ccc',
    borderRadius: 4,
    fontFamily: 'inherit',
    background: '#fff',
  },
  multiSelect: {
    fontSize: 13,
    padding: '4px',
    border: '1px solid #ccc',
    borderRadius: 4,
    fontFamily: 'inherit',
    background: '#fff',
    minHeight: 80,
  },
  checkboxLabel: {
    fontSize: 13,
    color: '#333',
    display: 'flex',
    alignItems: 'center',
    gap: 5,
    cursor: 'pointer',
    marginTop: 4,
  },
  footer: {
    display: 'flex',
    gap: 10,
    justifyContent: 'flex-end',
    marginTop: 20,
  },
  submitButton: {
    fontSize: 13,
    padding: '7px 18px',
    border: 'none',
    borderRadius: 4,
    background: '#1a1a1a',
    color: '#fff',
    cursor: 'pointer',
    fontFamily: 'inherit',
  },
  submitButtonDisabled: {
    fontSize: 13,
    padding: '7px 18px',
    border: 'none',
    borderRadius: 4,
    background: '#aaa',
    color: '#fff',
    cursor: 'not-allowed',
    fontFamily: 'inherit',
  },
  clearButton: {
    fontSize: 13,
    padding: '7px 18px',
    border: '1px solid #ccc',
    borderRadius: 4,
    background: '#fff',
    color: '#333',
    cursor: 'pointer',
    fontFamily: 'inherit',
  },
  errorText: {
    fontSize: 13,
    color: '#c0392b',
    marginTop: 10,
  },
  inlineError: {
    fontSize: 12,
    color: '#c0392b',
    marginTop: 2,
  },
};

export default function EnforcementDryRunForm() {
  const [tenantId, setTenantId] = useState('');
  const [actorId, setActorId] = useState('');
  const [actorType, setActorType] = useState('user');
  const [action, setAction] = useState('');
  const [resourceType, setResourceType] = useState('');
  const [resourceName, setResourceName] = useState('');
  const [resourceLocation, setResourceLocation] = useState('');
  const [sensitivity, setSensitivity] = useState(['internal']);
  const [pii, setPii] = useState(false);
  const [volume, setVolume] = useState('');
  const [authn, setAuthn] = useState('required');

  const [running, setRunning] = useState(false);
  const [result, setResult] = useState(null);
  const [submitError, setSubmitError] = useState(null);
  const [fieldErrors, setFieldErrors] = useState({});

  function getMultiSelectValues(e) {
    return Array.from(e.target.selectedOptions).map((o) => o.value);
  }

  function validate() {
    const errors = {};
    if (!tenantId.trim()) errors.tenantId = 'Tenant ID is required.';
    if (!actorId.trim()) errors.actorId = 'Actor ID is required.';
    if (!action.trim()) errors.action = 'Action is required.';
    if (!resourceType.trim()) errors.resourceType = 'Resource Type is required.';
    return errors;
  }

  async function handleSubmit(e) {
    e.preventDefault();
    setSubmitError(null);

    const errors = validate();
    if (Object.keys(errors).length > 0) {
      setFieldErrors(errors);
      return;
    }
    setFieldErrors({});

    const payload = {
      id: crypto.randomUUID(),
      schemaVersion: 'v1.3',
      tenantId: tenantId.trim(),
      timestamp: Date.now() / 1000,
      actor: {
        id: actorId.trim(),
        type: actorType,
      },
      action: action.trim(),
      resource: {
        type: resourceType.trim(),
        name: resourceName.trim() || undefined,
        location: resourceLocation.trim() || undefined,
      },
      data: {
        sensitivity: sensitivity,
        pii: pii,
        volume: volume || null,
      },
      risk: {
        authn: authn,
      },
      layer: null,
      tool_name: null,
      tool_method: null,
    };

    setRunning(true);
    try {
      const data = await runEnforce(payload);
      setResult(data);
    } catch (err) {
      setSubmitError(err.message);
    } finally {
      setRunning(false);
    }
  }

  function handleClear() {
    setResult(null);
    setSubmitError(null);
    setFieldErrors({});
  }

  return (
    <div style={styles.panel}>
      <div style={styles.panelTitle}>Enforcement Dry Run</div>
      <form onSubmit={handleSubmit} noValidate>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Intent</legend>
          <div style={styles.grid}>
            <div style={styles.field}>
              <label style={styles.label}>Tenant ID</label>
              <input
                style={styles.input}
                type="text"
                value={tenantId}
                onChange={(e) => setTenantId(e.target.value)}
                placeholder="tenant-id"
              />
              {fieldErrors.tenantId && <span style={styles.inlineError}>{fieldErrors.tenantId}</span>}
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Actor ID</label>
              <input
                style={styles.input}
                type="text"
                value={actorId}
                onChange={(e) => setActorId(e.target.value)}
                placeholder="actor-id"
              />
              {fieldErrors.actorId && <span style={styles.inlineError}>{fieldErrors.actorId}</span>}
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Actor Type</label>
              <select style={styles.select} value={actorType} onChange={(e) => setActorType(e.target.value)}>
                <option value="user">user</option>
                <option value="service">service</option>
                <option value="llm">llm</option>
                <option value="agent">agent</option>
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Action</label>
              <input
                style={styles.input}
                type="text"
                list="action-options"
                value={action}
                onChange={(e) => setAction(e.target.value)}
                placeholder="e.g. read"
              />
              <datalist id="action-options">
                <option value="read" />
                <option value="write" />
                <option value="update" />
                <option value="delete" />
                <option value="execute" />
                <option value="export" />
              </datalist>
              {fieldErrors.action && <span style={styles.inlineError}>{fieldErrors.action}</span>}
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Resource Type</label>
              <input
                style={styles.input}
                type="text"
                list="resource-type-options"
                value={resourceType}
                onChange={(e) => setResourceType(e.target.value)}
                placeholder="e.g. database"
              />
              <datalist id="resource-type-options">
                <option value="database" />
                <option value="storage" />
                <option value="api" />
                <option value="queue" />
                <option value="cache" />
              </datalist>
              {fieldErrors.resourceType && <span style={styles.inlineError}>{fieldErrors.resourceType}</span>}
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Resource Name</label>
              <input
                style={styles.input}
                type="text"
                value={resourceName}
                onChange={(e) => setResourceName(e.target.value)}
                placeholder="optional"
              />
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Resource Location</label>
              <input
                style={styles.input}
                type="text"
                value={resourceLocation}
                onChange={(e) => setResourceLocation(e.target.value)}
                placeholder="optional"
              />
            </div>
          </div>
        </fieldset>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Data & Risk</legend>
          <div style={styles.grid}>
            <div style={styles.field}>
              <label style={styles.label}>Data Sensitivity</label>
              <select
                style={styles.multiSelect}
                multiple
                value={sensitivity}
                onChange={(e) => setSensitivity(getMultiSelectValues(e))}
              >
                {SENSITIVITY_OPTIONS.map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
              <div style={styles.field}>
                <label style={styles.label}>Volume</label>
                <select style={styles.select} value={volume} onChange={(e) => setVolume(e.target.value)}>
                  <option value="">(none)</option>
                  <option value="single">single</option>
                  <option value="bulk">bulk</option>
                </select>
              </div>
              <div style={styles.field}>
                <label style={styles.label}>Risk: Authentication</label>
                <select style={styles.select} value={authn} onChange={(e) => setAuthn(e.target.value)}>
                  <option value="required">required</option>
                  <option value="not_required">not_required</option>
                </select>
              </div>
              <div style={styles.field}>
                <label style={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={pii}
                    onChange={(e) => setPii(e.target.checked)}
                  />
                  PII
                </label>
              </div>
            </div>
          </div>
        </fieldset>

        {submitError && <p style={styles.errorText}>{submitError}</p>}

        <div style={styles.footer}>
          <button
            type="button"
            style={styles.clearButton}
            onClick={handleClear}
          >
            Clear
          </button>
          <button
            type="submit"
            style={running ? styles.submitButtonDisabled : styles.submitButton}
            disabled={running}
          >
            {running ? 'Running...' : 'Run Enforce'}
          </button>
        </div>
      </form>

      <EnforcementResultPanel result={result} />
    </div>
  );
}
