import { useState } from 'react';
import { createPolicy } from '../api/policies';

const LAYERS = ['L0', 'L1', 'L2', 'L3', 'L4', 'L5', 'L6'];

const ACTION_OPTIONS = ['read', 'write', 'update', 'delete', 'execute', 'export'];
const ACTOR_TYPE_OPTIONS = ['user', 'service', 'llm', 'agent'];
const RESOURCE_TYPE_OPTIONS = ['database', 'storage', 'api', 'queue', 'cache'];
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
  gridFull: {
    display: 'grid',
    gridTemplateColumns: '1fr',
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
  sliderRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
  },
  slider: {
    flex: 1,
    accentColor: '#1a1a1a',
  },
  sliderValue: {
    fontSize: 13,
    color: '#555',
    minWidth: 34,
    textAlign: 'right',
  },
  checkboxGroup: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '8px 16px',
    paddingTop: 2,
  },
  checkboxLabel: {
    fontSize: 13,
    color: '#333',
    display: 'flex',
    alignItems: 'center',
    gap: 5,
    cursor: 'pointer',
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
  cancelButton: {
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

export default function PolicyForm({ onSuccess, onCancel }) {
  const [name, setName] = useState('');
  const [tenantId, setTenantId] = useState('');
  const [status, setStatus] = useState('active');
  const [type, setType] = useState('mandatory');
  const [layer, setLayer] = useState('');
  const [effect, setEffect] = useState('allow');
  const [decision, setDecision] = useState('min');
  const [thresholds, setThresholds] = useState({ action: 0.85, resource: 0.85, data: 0.85, risk: 0.85 });
  const [actions, setActions] = useState([]);
  const [actorTypes, setActorTypes] = useState([]);
  const [resourceTypes, setResourceTypes] = useState([]);
  const [sensitivity, setSensitivity] = useState([]);
  const [pii, setPii] = useState(false);
  const [volume, setVolume] = useState('');
  const [authn, setAuthn] = useState('required');

  const [saving, setSaving] = useState(false);
  const [submitError, setSubmitError] = useState(null);
  const [fieldErrors, setFieldErrors] = useState({});

  function setThreshold(key, value) {
    setThresholds((prev) => ({ ...prev, [key]: parseFloat(value) }));
  }

  function toggleActorType(value) {
    setActorTypes((prev) =>
      prev.includes(value) ? prev.filter((v) => v !== value) : [...prev, value]
    );
  }

  function getMultiSelectValues(e) {
    return Array.from(e.target.selectedOptions).map((o) => o.value);
  }

  function validate() {
    const errors = {};
    if (!name.trim()) errors.name = 'Name is required.';
    if (!tenantId.trim()) errors.tenantId = 'Tenant ID is required.';
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
      name: name.trim(),
      status,
      type,
      boundarySchemaVersion: 'v1.2',
      scope: { tenantId: tenantId.trim() },
      layer: layer || null,
      rules: {
        effect,
        thresholds: { ...thresholds },
        decision,
      },
      constraints: {
        action: {
          actions,
          actor_types: actorTypes,
        },
        resource: {
          types: resourceTypes,
          names: [],
          locations: [],
        },
        data: {
          sensitivity,
          pii,
          volume: volume || null,
        },
        risk: {
          authn,
        },
      },
    };

    setSaving(true);
    try {
      await createPolicy(payload);
      onSuccess();
    } catch (err) {
      setSubmitError(err.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={styles.panel}>
      <div style={styles.panelTitle}>New Policy</div>
      <form onSubmit={handleSubmit} noValidate>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Basic Info</legend>
          <div style={styles.grid}>
            <div style={styles.field}>
              <label style={styles.label}>Name</label>
              <input
                style={styles.input}
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Policy name"
              />
              {fieldErrors.name && <span style={styles.inlineError}>{fieldErrors.name}</span>}
            </div>
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
              <label style={styles.label}>Status</label>
              <select style={styles.select} value={status} onChange={(e) => setStatus(e.target.value)}>
                <option value="active">active</option>
                <option value="disabled">disabled</option>
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Type</label>
              <select style={styles.select} value={type} onChange={(e) => setType(e.target.value)}>
                <option value="mandatory">mandatory</option>
                <option value="optional">optional</option>
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Layer</label>
              <select style={styles.select} value={layer} onChange={(e) => setLayer(e.target.value)}>
                <option value="">(none)</option>
                {LAYERS.map((l) => (
                  <option key={l} value={l}>{l}</option>
                ))}
              </select>
            </div>
          </div>
        </fieldset>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Rules</legend>
          <div style={styles.grid}>
            <div style={styles.field}>
              <label style={styles.label}>Effect</label>
              <select style={styles.select} value={effect} onChange={(e) => setEffect(e.target.value)}>
                <option value="allow">allow</option>
                <option value="deny">deny</option>
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Decision</label>
              <select style={styles.select} value={decision} onChange={(e) => setDecision(e.target.value)}>
                <option value="min">min</option>
                <option value="weighted-avg">weighted-avg</option>
              </select>
            </div>
          </div>
          <div style={{ marginTop: 14, display: 'flex', flexDirection: 'column', gap: 10 }}>
            {['action', 'resource', 'data', 'risk'].map((key) => (
              <div key={key} style={styles.field}>
                <label style={styles.label}>Threshold: {key}</label>
                <div style={styles.sliderRow}>
                  <input
                    style={styles.slider}
                    type="range"
                    min={0}
                    max={1}
                    step={0.01}
                    value={thresholds[key]}
                    onChange={(e) => setThreshold(key, e.target.value)}
                  />
                  <span style={styles.sliderValue}>{thresholds[key].toFixed(2)}</span>
                </div>
              </div>
            ))}
          </div>
        </fieldset>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Constraints</legend>
          <div style={styles.grid}>
            <div style={styles.field}>
              <label style={styles.label}>Actions</label>
              <select
                style={styles.multiSelect}
                multiple
                value={actions}
                onChange={(e) => setActions(getMultiSelectValues(e))}
              >
                {ACTION_OPTIONS.map((a) => (
                  <option key={a} value={a}>{a}</option>
                ))}
              </select>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Actor Types</label>
              <div style={styles.checkboxGroup}>
                {ACTOR_TYPE_OPTIONS.map((a) => (
                  <label key={a} style={styles.checkboxLabel}>
                    <input
                      type="checkbox"
                      checked={actorTypes.includes(a)}
                      onChange={() => toggleActorType(a)}
                    />
                    {a}
                  </label>
                ))}
              </div>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Resource Types</label>
              <select
                style={styles.multiSelect}
                multiple
                value={resourceTypes}
                onChange={(e) => setResourceTypes(getMultiSelectValues(e))}
              >
                {RESOURCE_TYPE_OPTIONS.map((r) => (
                  <option key={r} value={r}>{r}</option>
                ))}
              </select>
            </div>
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
        </fieldset>

        {submitError && <p style={styles.errorText}>{submitError}</p>}

        <div style={styles.footer}>
          <button
            type="button"
            style={styles.cancelButton}
            onClick={onCancel}
            disabled={saving}
          >
            Cancel
          </button>
          <button
            type="submit"
            style={saving ? styles.submitButtonDisabled : styles.submitButton}
            disabled={saving}
          >
            {saving ? 'Saving...' : 'Create Policy'}
          </button>
        </div>
      </form>
    </div>
  );
}
