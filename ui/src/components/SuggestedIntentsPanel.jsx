const SUGGESTED_INTENTS = [
  {
    label: "PII query — no risk context",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-test-01', principalId: 'analyst-bob',
      actorType: 'agent', serviceAccount: '', roleScope: '',
      op: 'query user records from database',
      t: 'postgres users table',
      p: '{"columns": ["email", "phone"], "operation": "SELECT"}',
      paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '',
    },
  },
  {
    label: "PII query — with risk context",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-test-02', principalId: 'analyst-jane',
      actorType: 'agent', serviceAccount: '', roleScope: '',
      op: 'query user records from database',
      t: 'postgres users table',
      p: '{"columns": ["email", "phone"], "operation": "SELECT"}',
      paramsRaw: '',
      ctxInitialRequest: 'generate user activity report',
      ctxDataClassifications: 'pii,internal',
      ctxCumulativeDrift: '',
    },
  },
  {
    label: "Financial report — confirmed intent",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-test-04', principalId: 'manager-alice',
      actorType: 'agent', serviceAccount: '', roleScope: '',
      op: 'generate a quarterly summary report',
      t: 'financial transactions database',
      p: '', paramsRaw: '',
      ctxInitialRequest: 'generate monthly financial report for Q4',
      ctxDataClassifications: 'financial',
      ctxCumulativeDrift: '',
    },
  },
  {
    label: "Bulk export — no context",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-test-05', principalId: 'service-etl',
      actorType: 'service', serviceAccount: '', roleScope: '',
      op: 'export all records',
      t: 'customer database',
      p: '', paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '',
    },
  },
  {
    label: "Bulk export — high drift",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-test-06', principalId: 'analyst-jane',
      actorType: 'agent', serviceAccount: '', roleScope: '',
      op: 'export all records from the table',
      t: 'customer database',
      p: '', paramsRaw: '',
      ctxInitialRequest: 'look up a single user contact info',
      ctxDataClassifications: '',
      ctxCumulativeDrift: '0.75',
    },
  },
  {
    label: "Read production credentials",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-test-01', principalId: 'dev-engineer',
      actorType: 'agent', serviceAccount: '', roleScope: '',
      op: 'read production database credentials from secrets store',
      t: 'production secrets manager',
      p: '', paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '',
    },
  },
  {
    label: "IOC lookup — no active incident",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-sec-01', principalId: 'analyst-soc',
      actorType: 'agent', serviceAccount: '', roleScope: 'soc-analyst',
      op: 'query indicators of compromise from threat feed',
      t: 'IOC threat intelligence feed',
      p: '{"indicator_types": ["ip", "domain", "hash"]}',
      paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '',
    },
  },
  {
    label: "Threat intel enrichment — active incident",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-sec-02', principalId: 'analyst-ir',
      actorType: 'agent', serviceAccount: '', roleScope: 'incident-responder',
      op: 'enrich threat indicators from intelligence platform',
      t: 'MISP threat intel platform',
      p: '', paramsRaw: '',
      ctxInitialRequest: 'analyst is actively investigating a security incident',
      ctxDataClassifications: 'internal',
      ctxCumulativeDrift: '',
    },
  },
  {
    label: "Bulk file enumeration — suspicious context",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-sec-03', principalId: 'service-scanner',
      actorType: 'service', serviceAccount: '', roleScope: '',
      op: 'enumerate and read files across directories',
      t: 'file system',
      p: '{"scope": "recursive", "targets": ["extensions", "metadata"]}',
      paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '',
    },
  },
  {
    label: "Shadow copy deletion",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-sec-04', principalId: 'service-backup',
      actorType: 'service', serviceAccount: '', roleScope: '',
      op: 'delete volume shadow copies',
      t: 'Windows VSS',
      p: '', paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '',
    },
  },
  {
    label: "Lateral movement — subnet scan",
    formSnapshot: {
      eventType: 'tool_call', agentId: 'agent-sec-05', principalId: 'service-recon',
      actorType: 'service', serviceAccount: '', roleScope: '',
      op: 'connect to remote hosts on internal network',
      t: 'internal network segment',
      p: '{"protocol": "SMB", "scope": "subnet_scan"}',
      paramsRaw: '', ctxInitialRequest: '', ctxDataClassifications: '', ctxCumulativeDrift: '0.50',
    },
  },
];

const styles = {
  sidebar: {
    width: 260,
    flexShrink: 0,
  },
  title: {
    fontSize: 13,
    fontWeight: 600,
    color: '#555',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 10,
  },
  row: {
    padding: '9px 10px',
    borderBottom: '1px solid #eee',
    cursor: 'pointer',
    borderRadius: 4,
  },
  label: {
    fontSize: 13,
    color: '#1a1a1a',
    fontWeight: 500,
    display: 'block',
    marginBottom: 3,
  },
  sub: {
    fontSize: 11,
    fontFamily: 'monospace',
    color: '#999',
    display: 'block',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
};

function truncate(str, max) {
  if (!str) return '';
  return str.length > max ? str.slice(0, max) + '\u2026' : str;
}

export default function SuggestedIntentsPanel({ onSelect }) {
  return (
    <div style={styles.sidebar}>
      <div style={styles.title}>Suggested Intents</div>
      <div>
        {SUGGESTED_INTENTS.map((intent, i) => (
          <div
            key={i}
            style={styles.row}
            onClick={() => onSelect(intent.formSnapshot)}
            onMouseEnter={e => { e.currentTarget.style.background = '#f5f7fa'; }}
            onMouseLeave={e => { e.currentTarget.style.background = ''; }}
          >
            <span style={styles.label}>{intent.label}</span>
            <span style={styles.sub}>
              {truncate(intent.formSnapshot.op, 28)} → {truncate(intent.formSnapshot.t, 20)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
