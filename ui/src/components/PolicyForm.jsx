import { useState } from 'react';
import { createPolicy, updatePolicy } from '../api/policies';

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
  textarea: {
    fontSize: 13,
    padding: '6px 10px',
    border: '1px solid #ccc',
    borderRadius: 4,
    fontFamily: 'inherit',
    background: '#fff',
    resize: 'vertical',
    minHeight: 72,
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
  hint: {
    display: 'block',
    fontSize: '11px',
    color: '#888',
    marginTop: '3px',
    lineHeight: '1.4',
  },
  labelRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
  },
  modeToggle: {
    fontSize: 11,
    color: '#888',
    cursor: 'pointer',
    userSelect: 'none',
    marginLeft: 'auto',
  },
  modeToggleActive: {
    fontWeight: 700,
    color: '#1a1a1a',
  },
  inputInvalid: {
    fontSize: 13,
    padding: '6px 10px',
    border: '1px solid #c0392b',
    borderRadius: 4,
    fontFamily: 'inherit',
    background: '#fff',
    resize: 'vertical',
    minHeight: 72,
  },
};

export default function PolicyForm({ onSuccess, onCancel, policy = null }) {
  const isEdit = policy !== null;

  const [name, setName] = useState(isEdit ? policy.name : '');
  const [tenantId, setTenantId] = useState(isEdit ? policy.tenant_id : '');
  const [status, setStatus] = useState(isEdit ? policy.status : 'active');
  const [policyType, setPolicyType] = useState(isEdit ? policy.policy_type : 'forbidden');
  const [priority, setPriority] = useState(isEdit ? policy.priority : 0);

  const [matchOp, setMatchOp] = useState(isEdit ? (policy.match?.op ?? '') : '');
  const [matchT, setMatchT] = useState(isEdit ? (policy.match?.t ?? '') : '');
  const [matchP, setMatchP] = useState(isEdit ? (policy.match?.p ?? '') : '');
  const [matchCtx, setMatchCtx] = useState(isEdit ? (policy.match?.ctx ?? '') : '');

  const [thresholds, setThresholds] = useState(
    isEdit && policy.thresholds
      ? { action: 0.85, resource: 0.85, data: 0.85, risk: 0.85, ...policy.thresholds }
      : { action: 0.85, resource: 0.85, data: 0.85, risk: 0.85 }
  );
  const [scoringMode, setScoringMode] = useState(
    isEdit && policy.weights ? 'weighted-avg' : 'min'
  );
  const [weights, setWeights] = useState(
    isEdit && policy.weights
      ? { action: 1.0, resource: 1.0, data: 1.0, risk: 1.0, ...policy.weights }
      : { action: 1.0, resource: 1.0, data: 1.0, risk: 1.0 }
  );

  const [driftThreshold, setDriftThreshold] = useState(
    isEdit && policy.drift_threshold != null ? String(policy.drift_threshold) : ''
  );
  const [notes, setNotes] = useState(isEdit ? (policy.notes ?? '') : '');

  const [jsonMode, setJsonMode] = useState({ op: false, t: false, p: false, ctx: false });
  const [jsonErrors, setJsonErrors] = useState({ op: null, t: null, p: null, ctx: null });

  const [saving, setSaving] = useState(false);
  const [submitError, setSubmitError] = useState(null);
  const [fieldErrors, setFieldErrors] = useState({});

  function setThreshold(key, value) {
    setThresholds((prev) => ({ ...prev, [key]: parseFloat(value) }));
  }

  function setWeight(key, value) {
    setWeights((prev) => ({ ...prev, [key]: parseFloat(value) }));
  }

  function toggleJsonMode(field) {
    setJsonMode((prev) => ({ ...prev, [field]: !prev[field] }));
    setJsonErrors((prev) => ({ ...prev, [field]: null }));
  }

  function handleAnchorChange(field, value, setter) {
    setter(value);
    if (jsonMode[field]) {
      try {
        JSON.parse(value);
        setJsonErrors((prev) => ({ ...prev, [field]: null }));
      } catch {
        setJsonErrors((prev) => ({ ...prev, [field]: 'Invalid JSON' }));
      }
    }
  }

  function validate() {
    const errors = {};
    if (!name.trim()) errors.name = 'Name is required.';
    if (!tenantId.trim()) errors.tenantId = 'Tenant ID is required.';
    if (!matchOp.trim()) errors.matchOp = 'Operation is required.';
    if (!matchT.trim()) errors.matchT = 'Target / Tool is required.';
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

    const now = Date.now() / 1000;

    const match = {
      op: matchOp.trim(),
      t: matchT.trim(),
    };
    if (matchP.trim()) match.p = matchP.trim();
    if (matchCtx.trim()) match.ctx = matchCtx.trim();

    const payload = isEdit
      ? {
          id: policy.id,
          name: name.trim(),
          tenant_id: tenantId.trim(),
          status,
          policy_type: policyType,
          priority,
          match,
          thresholds: { ...thresholds },
          scoring_mode: scoringMode,
          weights: scoringMode === 'weighted-avg' ? { ...weights } : null,
          drift_threshold: driftThreshold !== '' ? parseFloat(driftThreshold) : null,
          notes: notes.trim() || null,
          created_at: policy.created_at,
          updated_at: now,
        }
      : {
          id: crypto.randomUUID(),
          name: name.trim(),
          tenant_id: tenantId.trim(),
          status,
          policy_type: policyType,
          priority,
          match,
          thresholds: { ...thresholds },
          scoring_mode: scoringMode,
          weights: scoringMode === 'weighted-avg' ? { ...weights } : null,
          drift_threshold: driftThreshold !== '' ? parseFloat(driftThreshold) : null,
          notes: notes.trim() || null,
          created_at: now,
          updated_at: now,
        };

    setSaving(true);
    try {
      if (isEdit) {
        await updatePolicy(policy.id, payload);
      } else {
        await createPolicy(payload);
      }
      onSuccess();
    } catch (err) {
      setSubmitError(err.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={styles.panel}>
      <div style={styles.panelTitle}>{isEdit ? 'Edit Policy' : 'New Policy'}</div>
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
              <small style={styles.hint}>A human-readable label for this policy boundary.</small>
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
              <small style={styles.hint}>The tenant this policy applies to. Must match the tenant_id in incoming intent events.</small>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Status</label>
              <select style={styles.select} value={status} onChange={(e) => setStatus(e.target.value)}>
                <option value="active">active</option>
                <option value="disabled">disabled</option>
              </select>
              <small style={styles.hint}>Active policies are evaluated during enforcement. Disabled policies are stored but skipped.</small>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Policy Type</label>
              <select style={styles.select} value={policyType} onChange={(e) => setPolicyType(e.target.value)}>
                <option value="forbidden">forbidden</option>
                <option value="context_allow">context_allow</option>
                <option value="context_deny">context_deny</option>
                <option value="context_defer">context_defer</option>
              </select>
              <small style={styles.hint}>forbidden: always blocks regardless of context. context_allow: denied by default, allowed when context confirms intent. context_deny: allowed by default, blocked when context signals risk. context_defer: triggers DEFER when action is ambiguous or context is insufficient.</small>
            </div>
            <div style={styles.field}>
              <label style={styles.label}>Priority</label>
              <input
                style={styles.input}
                type="number"
                value={priority}
                onChange={(e) => setPriority(parseInt(e.target.value, 10) || 0)}
                step={1}
              />
              <small style={styles.hint}>Lower number = higher priority. When multiple policies match, the lowest priority number wins.</small>
            </div>
          </div>
        </fieldset>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Match Anchors</legend>
          <small style={{ ...styles.hint, marginBottom: 12 }}>Natural language descriptions of the action pattern this policy should match. The engine embeds these as semantic vectors and compares them against incoming intent events.</small>
          <div style={styles.gridFull}>
            <div style={styles.field}>
              <div style={styles.labelRow}>
                <label style={styles.label}>Operation</label>
                <span style={styles.modeToggle}>
                  <span
                    style={!jsonMode.op ? styles.modeToggleActive : {}}
                    onClick={() => jsonMode.op && toggleJsonMode('op')}
                  >NL</span>
                  {' | '}
                  <span
                    style={jsonMode.op ? styles.modeToggleActive : {}}
                    onClick={() => !jsonMode.op && toggleJsonMode('op')}
                  >JSON</span>
                </span>
              </div>
              {jsonMode.op ? (
                <textarea
                  style={jsonErrors.op ? styles.inputInvalid : { ...styles.textarea }}
                  value={matchOp}
                  onChange={(e) => handleAnchorChange('op', e.target.value, setMatchOp)}
                  placeholder='e.g. {"action": "read", "scope": "users"}'
                />
              ) : (
                <input
                  style={styles.input}
                  type="text"
                  value={matchOp}
                  onChange={(e) => setMatchOp(e.target.value)}
                  placeholder="e.g. read user records from database"
                />
              )}
              {fieldErrors.matchOp && <span style={styles.inlineError}>{fieldErrors.matchOp}</span>}
              {jsonErrors.op && <span style={styles.inlineError}>{jsonErrors.op}</span>}
              <small style={styles.hint}>Describe the action being performed. E.g. 'query a database', 'send an email', 'read a file'.</small>
            </div>
            <div style={styles.field}>
              <div style={styles.labelRow}>
                <label style={styles.label}>Target / Tool</label>
                <span style={styles.modeToggle}>
                  <span
                    style={!jsonMode.t ? styles.modeToggleActive : {}}
                    onClick={() => jsonMode.t && toggleJsonMode('t')}
                  >NL</span>
                  {' | '}
                  <span
                    style={jsonMode.t ? styles.modeToggleActive : {}}
                    onClick={() => !jsonMode.t && toggleJsonMode('t')}
                  >JSON</span>
                </span>
              </div>
              {jsonMode.t ? (
                <textarea
                  style={jsonErrors.t ? styles.inputInvalid : { ...styles.textarea }}
                  value={matchT}
                  onChange={(e) => handleAnchorChange('t', e.target.value, setMatchT)}
                  placeholder='e.g. {"tool": "postgres", "table": "users"}'
                />
              ) : (
                <input
                  style={styles.input}
                  type="text"
                  value={matchT}
                  onChange={(e) => setMatchT(e.target.value)}
                  placeholder="e.g. postgres users table"
                />
              )}
              {fieldErrors.matchT && <span style={styles.inlineError}>{fieldErrors.matchT}</span>}
              {jsonErrors.t && <span style={styles.inlineError}>{jsonErrors.t}</span>}
              <small style={styles.hint}>Describe the resource or tool being accessed. E.g. 'postgres users table', 'Gmail API', 'S3 bucket'.</small>
            </div>
            <div style={styles.field}>
              <div style={styles.labelRow}>
                <label style={styles.label}>Parameters — optional</label>
                <span style={styles.modeToggle}>
                  <span
                    style={!jsonMode.p ? styles.modeToggleActive : {}}
                    onClick={() => jsonMode.p && toggleJsonMode('p')}
                  >NL</span>
                  {' | '}
                  <span
                    style={jsonMode.p ? styles.modeToggleActive : {}}
                    onClick={() => !jsonMode.p && toggleJsonMode('p')}
                  >JSON</span>
                </span>
              </div>
              {jsonMode.p ? (
                <textarea
                  style={jsonErrors.p ? styles.inputInvalid : { ...styles.textarea }}
                  value={matchP}
                  onChange={(e) => handleAnchorChange('p', e.target.value, setMatchP)}
                  placeholder='e.g. {"columns": ["email", "name"]}'
                />
              ) : (
                <input
                  style={styles.input}
                  type="text"
                  value={matchP}
                  onChange={(e) => setMatchP(e.target.value)}
                  placeholder="e.g. query includes email and name columns"
                />
              )}
              {jsonErrors.p && <span style={styles.inlineError}>{jsonErrors.p}</span>}
              <small style={styles.hint}>Optional. Describe the parameter pattern to match. E.g. 'queries containing personal identifiers'. Leave blank to match any parameters.</small>
            </div>
            <div style={styles.field}>
              <div style={styles.labelRow}>
                <label style={styles.label}>Risk Context — optional</label>
                <span style={styles.modeToggle}>
                  <span
                    style={!jsonMode.ctx ? styles.modeToggleActive : {}}
                    onClick={() => jsonMode.ctx && toggleJsonMode('ctx')}
                  >NL</span>
                  {' | '}
                  <span
                    style={jsonMode.ctx ? styles.modeToggleActive : {}}
                    onClick={() => !jsonMode.ctx && toggleJsonMode('ctx')}
                  >JSON</span>
                </span>
              </div>
              {jsonMode.ctx ? (
                <textarea
                  style={jsonErrors.ctx ? styles.inputInvalid : { ...styles.textarea }}
                  value={matchCtx}
                  onChange={(e) => handleAnchorChange('ctx', e.target.value, setMatchCtx)}
                  placeholder='e.g. {"signal": "pii_access", "window_minutes": 5}'
                />
              ) : (
                <input
                  style={styles.input}
                  type="text"
                  value={matchCtx}
                  onChange={(e) => setMatchCtx(e.target.value)}
                  placeholder="e.g. accessed PII in the last 5 minutes"
                />
              )}
              {jsonErrors.ctx && <span style={styles.inlineError}>{jsonErrors.ctx}</span>}
              <small style={styles.hint}>Optional. Describe the session context signal this policy reacts to. E.g. 'requests involving financial data'. Leave blank to ignore context.</small>
            </div>
          </div>
        </fieldset>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Thresholds &amp; Scoring</legend>

          <div style={{ ...styles.field, marginBottom: 16 }}>
            <label style={styles.label}>Scoring Mode</label>
            <select style={styles.select} value={scoringMode} onChange={(e) => setScoringMode(e.target.value)}>
              <option value="min">min</option>
              <option value="weighted-avg">weighted-avg</option>
            </select>
            <small style={styles.hint}>
              'min': each slice must independently exceed its own threshold — one failure blocks the match.<br />
              'weighted-avg': similarities and thresholds are both weight-averaged into single scores, then compared — slices with higher weight have more influence on the outcome.
            </small>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0 24px' }}>

            <div>
              <div style={{ fontSize: 11, fontWeight: 600, color: '#555', textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 10 }}>Thresholds</div>
              <small style={{ ...styles.hint, marginBottom: 12, display: 'block' }}>Minimum similarity per slice (0.0–1.0). Active in both modes — in weighted-avg they are averaged together with the weights.</small>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
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
                    <small style={styles.hint}>{key === 'action' ? 'Operation/action slice.' : key === 'resource' ? 'Target/resource slice.' : key === 'data' ? 'Parameters/data slice.' : 'Risk context slice.'}</small>
                  </div>
                ))}
              </div>
            </div>

            <div style={scoringMode === 'min' ? { opacity: 0.35, pointerEvents: 'none' } : {}}>
              <div style={{ fontSize: 11, fontWeight: 600, color: '#555', textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 10 }}>
                Weights {scoringMode === 'min' && <span style={{ fontWeight: 400, textTransform: 'none', letterSpacing: 0 }}>(not used in min mode)</span>}
              </div>
              <small style={{ ...styles.hint, marginBottom: 12, display: 'block' }}>Relative influence of each slice in weighted-avg mode. Higher weight = that slice pulls the combined score and threshold more.</small>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                {['action', 'resource', 'data', 'risk'].map((key) => (
                  <div key={key} style={styles.field}>
                    <label style={styles.label}>Weight: {key}</label>
                    <div style={styles.sliderRow}>
                      <input
                        style={styles.slider}
                        type="range"
                        min={0}
                        max={2}
                        step={0.1}
                        value={weights[key]}
                        onChange={(e) => setWeight(key, e.target.value)}
                      />
                      <span style={styles.sliderValue}>{weights[key].toFixed(1)}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

          </div>
        </fieldset>

        <fieldset style={styles.fieldset}>
          <legend style={styles.legend}>Advanced</legend>
          <div style={styles.grid}>
            <div style={styles.field}>
              <label style={styles.label}>Drift Threshold — optional (0.0–1.0)</label>
              <input
                style={styles.input}
                type="number"
                value={driftThreshold}
                onChange={(e) => setDriftThreshold(e.target.value)}
                placeholder="e.g. 0.3"
                min={0}
                max={1}
                step={0.01}
              />
              <small style={styles.hint}>Optional. If the semantic distance between the agent's current action and the user's original request exceeds this value (0.0–1.0), the policy triggers a DEFER or STEP_UP. Leave blank to disable drift enforcement for this policy.</small>
            </div>
          </div>
          <div style={{ ...styles.gridFull, marginTop: 14 }}>
            <div style={styles.field}>
              <label style={styles.label}>Notes — optional</label>
              <textarea
                style={styles.textarea}
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="Additional context or notes about this policy"
              />
              <small style={styles.hint}>Optional free-text notes for your own reference. Not used during enforcement.</small>
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
            {saving ? 'Saving...' : isEdit ? 'Update Policy' : 'Create Policy'}
          </button>
        </div>
      </form>
    </div>
  );
}
