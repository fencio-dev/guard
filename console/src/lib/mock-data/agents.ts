export interface AgentRun {
  id: string;
  agentName: string;
  status: 'success' | 'running' | 'failed';
  started: Date;
  duration: number; // milliseconds
  policiesApplied: number;
  trace: string[];
}

export const mockAgentRuns: AgentRun[] = [
  {
    id: 'run_1a2b3c4d',
    agentName: 'ContentModerator',
    status: 'success',
    started: new Date(Date.now() - 2 * 60 * 1000), // 2 minutes ago
    duration: 1200,
    policiesApplied: 2,
    trace: [
      '[2025-11-18 10:30:00] Agent initialized',
      '[2025-11-18 10:30:01] Processing user input',
      '[2025-11-18 10:30:01] Checking against L4 policy family',
      '[2025-11-18 10:30:01] Policy check passed',
      '[2025-11-18 10:30:02] Execution completed successfully'
    ]
  },
  {
    id: 'run_5e6f7g8h',
    agentName: 'DataAnalyzer',
    status: 'running',
    started: new Date(Date.now() - 30 * 1000), // 30 seconds ago
    duration: 0,
    policiesApplied: 3,
    trace: [
      '[2025-11-18 10:32:30] Agent initialized',
      '[2025-11-18 10:32:31] Processing data request',
      '[2025-11-18 10:32:32] Checking against L4 policy family',
      '[2025-11-18 10:32:33] Running analysis...'
    ]
  },
  {
    id: 'run_9i0j1k2l',
    agentName: 'CodeReviewer',
    status: 'failed',
    started: new Date(Date.now() - 15 * 60 * 1000), // 15 minutes ago
    duration: 3400,
    policiesApplied: 1,
    trace: [
      '[2025-11-18 10:15:00] Agent initialized',
      '[2025-11-18 10:15:01] Processing code submission',
      '[2025-11-18 10:15:02] Checking against L4 policy family',
      '[2025-11-18 10:15:03] ERROR: Policy violation detected',
      '[2025-11-18 10:15:03] Blocking execution due to security policy'
    ]
  },
  {
    id: 'run_3m4n5o6p',
    agentName: 'ContentModerator',
    status: 'success',
    started: new Date(Date.now() - 60 * 60 * 1000), // 1 hour ago
    duration: 980,
    policiesApplied: 2,
    trace: [
      '[2025-11-18 09:30:00] Agent initialized',
      '[2025-11-18 09:30:00] Processing user input',
      '[2025-11-18 09:30:01] Checking against L4 policy family',
      '[2025-11-18 09:30:01] Policy check passed',
      '[2025-11-18 09:30:01] Execution completed successfully'
    ]
  },
  {
    id: 'run_7q8r9s0t',
    agentName: 'ReportGenerator',
    status: 'success',
    started: new Date(Date.now() - 2 * 60 * 60 * 1000), // 2 hours ago
    duration: 5600,
    policiesApplied: 1,
    trace: [
      '[2025-11-18 08:30:00] Agent initialized',
      '[2025-11-18 08:30:01] Processing report request',
      '[2025-11-18 08:30:02] Checking against L4 policy family',
      '[2025-11-18 08:30:03] Generating report',
      '[2025-11-18 08:30:08] Report generation complete'
    ]
  }
];
