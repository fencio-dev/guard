import { Pin, PinOff, Trash2 } from 'lucide-react';

const BADGE_COLORS = {
  ALLOW:   { background: '#d4edda', color: '#155724' },
  DENY:    { background: '#f8d7da', color: '#721c24' },
  MODIFY:  { background: '#fff3cd', color: '#856404' },
  STEP_UP: { background: '#cce5ff', color: '#004085' },
  DEFER:   { background: '#e2e3e5', color: '#383d41' },
};

const styles = {
  section: {
    borderTop: '2px solid #e8e8e8',
    marginTop: 28,
    paddingTop: 24,
  },
  title: {
    fontSize: 14,
    fontWeight: 600,
    color: '#333',
    marginBottom: 12,
  },
  row: {
    display: 'flex',
    alignItems: 'center',
    gap: 12,
    padding: '8px 10px',
    borderBottom: '1px solid #eee',
    cursor: 'pointer',
  },
  rowPinned: {
    display: 'flex',
    alignItems: 'center',
    gap: 12,
    padding: '8px 10px',
    borderBottom: '1px solid #eee',
    cursor: 'pointer',
    background: '#f5f9ff',
  },
  opText: {
    fontSize: 13,
    fontFamily: 'monospace',
    color: '#1a1a1a',
    flex: '0 0 auto',
    maxWidth: 280,
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  sep: {
    fontSize: 13,
    color: '#bbb',
    flex: '0 0 auto',
  },
  tText: {
    fontSize: 13,
    fontFamily: 'monospace',
    color: '#555',
    flex: '1 1 auto',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    minWidth: 0,
  },
  badge: {
    fontSize: 11,
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: 4,
    flex: '0 0 auto',
    letterSpacing: 0.3,
  },
  pinButton: {
    background: 'none',
    border: 'none',
    cursor: 'pointer',
    padding: '0 4px',
    flex: '0 0 auto',
    lineHeight: 1,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    color: '#687385',
  },
  pinButtonPinned: {
    color: '#2563eb',
  },
};

function truncate(str, max) {
  if (!str) return '';
  return str.length > max ? str.slice(0, max) + '…' : str;
}

export default function RecentRunsPanel({ runs, pinnedIndex, onSelect, onPin, onClear }) {
  if (!runs || runs.length === 0) return null;

  // Build display list: pinned entry first (if any), then the rest in order.
  // Each item carries its original index so onPin receives the correct value.
  let displayList;
  if (pinnedIndex !== null && pinnedIndex >= 0 && pinnedIndex < runs.length) {
    const pinned = { ...runs[pinnedIndex], originalIndex: pinnedIndex, isPinned: true };
    const rest = runs
      .map((r, i) => ({ ...r, originalIndex: i, isPinned: false }))
      .filter((_, i) => i !== pinnedIndex);
    displayList = [pinned, ...rest];
  } else {
    displayList = runs.map((r, i) => ({ ...r, originalIndex: i, isPinned: false }));
  }

  return (
    <div style={styles.section}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 12 }}>
        <div style={{ ...styles.title, marginBottom: 0 }}>Recent Runs</div>
        {onClear && (
          <button
            style={{ background: 'none', border: 'none', cursor: 'pointer', display: 'inline-flex', alignItems: 'center', gap: 4, color: '#999', fontSize: 12, padding: '2px 4px' }}
            title="Clear recent runs"
            onClick={onClear}
          >
            <Trash2 size={13} />
            Clear
          </button>
        )}
      </div>
      <div>
        {displayList.map((item) => {
          const badgeColors = BADGE_COLORS[item.decision] ?? BADGE_COLORS.DEFER;
          const rowStyle = item.isPinned ? styles.rowPinned : styles.row;
          const originalIndex = item.originalIndex;

          return (
            <div
              key={originalIndex}
              style={rowStyle}
              onClick={() => onSelect(item.formSnapshot)}
            >
              <span style={styles.opText} title={item.formSnapshot.op}>
                {truncate(item.formSnapshot.op, 40)}
              </span>
              <span style={styles.sep}>→</span>
              <span style={styles.tText} title={item.formSnapshot.t}>
                {truncate(item.formSnapshot.t, 30)}
              </span>
              <span style={{ ...styles.badge, ...badgeColors }}>
                {item.decision ?? '—'}
              </span>
              <button
                style={item.isPinned ? { ...styles.pinButton, ...styles.pinButtonPinned } : styles.pinButton}
                title={item.isPinned ? 'Unpin' : 'Pin'}
                onClick={(e) => {
                  e.stopPropagation();
                  onPin(originalIndex);
                }}
              >
                {item.isPinned ? <Pin size={14} /> : <PinOff size={14} />}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
