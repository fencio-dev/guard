import { useState } from 'react';
import PolicyList from './components/PolicyList';
import EnforcementDryRunForm from './components/EnforcementDryRunForm';
import TelemetryTable from './components/TelemetryTable';

const TABS = ['Policies', 'Dry Run', 'Telemetry'];

export default function App() {
  const [activeTab, setActiveTab] = useState('Policies');
  const [tenantId, setTenantId] = useState(
    () => localStorage.getItem('guardTenantId') || ''
  );

  function handleTenantChange(e) {
    const val = e.target.value;
    setTenantId(val);
    localStorage.setItem('guardTenantId', val);
  }

  return (
    <div className="container">
      <header className="header">
        <span className="header-title">Guard</span>
        <nav className="tab-bar">
          {TABS.map((tab) => (
            <button
              key={tab}
              className={`tab-button${activeTab === tab ? ' active' : ''}`}
              onClick={() => setActiveTab(tab)}
            >
              {tab}
            </button>
          ))}
        </nav>
        <div className="header-tenant">
          <label htmlFor="tenant-id-input">Tenant ID</label>
          <input
            id="tenant-id-input"
            type="text"
            value={tenantId}
            onChange={handleTenantChange}
            placeholder="your-tenant-id"
            spellCheck={false}
          />
        </div>
      </header>
      <main className="content">
        <div className="tab-panel">
          {activeTab === 'Policies' && <PolicyList />}
          {activeTab === 'Dry Run' && <EnforcementDryRunForm />}
          {activeTab === 'Telemetry' && <TelemetryTable />}
        </div>
      </main>
    </div>
  );
}
