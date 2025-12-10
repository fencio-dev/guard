import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { GradientCard } from "@/components/ui/gradient-card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { fetchAgentRuns, type SessionSummary } from "@/lib/telemetry-api";
import { motion } from "framer-motion";
import { fadeUp, scaleReveal, staggerContainer } from "@/lib/animations";

const AgentsIndexPage = () => {
  const navigate = useNavigate();

  // State for filters
  const [agentIdFilter, setAgentIdFilter] = useState("");
  const [decisionFilter, setDecisionFilter] = useState<0 | 1 | undefined>(undefined);
  const [currentPage, setCurrentPage] = useState(0);
  const [limit] = useState(50);

  // State for data
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch data function
  const loadSessions = async () => {
    try {
      setIsLoading(true);
      setError(null);

      const response = await fetchAgentRuns({
        agentId: agentIdFilter || undefined,
        decision: decisionFilter,
        limit,
        offset: currentPage * limit,
      });

      setSessions(response.sessions);
      setTotalCount(response.totalCount);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load sessions');
      console.error('Failed to fetch sessions:', err);
    } finally {
      setIsLoading(false);
    }
  };

  // Load data on mount and when filters change
  useEffect(() => {
    loadSessions();
  }, [agentIdFilter, decisionFilter, currentPage]);

  // Auto-refresh every 5 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      loadSessions();
    }, 5000);

    return () => clearInterval(interval);
  }, [agentIdFilter, decisionFilter, currentPage]);

  const formatDuration = (duration_us: number) => {
    const ms = duration_us / 1000;
    if (ms < 1000) return `${ms.toFixed(0)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatRelativeTime = (timestamp_ms: number) => {
    const seconds = Math.floor((Date.now() - timestamp_ms) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  };

  const getDecisionColor = (decision: 0 | 1) => {
    return decision === 1
      ? 'bg-green-500/10 text-green-700 dark:text-green-400'
      : 'bg-red-500/10 text-red-700 dark:text-red-400';
  };

  const getDecisionLabel = (decision: 0 | 1) => {
    return decision === 1 ? 'ALLOW' : 'BLOCK';
  };

  const totalPages = Math.ceil(totalCount / limit);

  return (
    <motion.div
      variants={staggerContainer}
      initial="hidden"
      animate="show"
    >
      <motion.div
        className="flex justify-between items-center mb-6"
        variants={fadeUp}
      >
        <h1 className="text-4xl font-bold bg-gradient-hero bg-clip-text text-transparent">
          Agent Enforcement Sessions
        </h1>
      </motion.div>

      {/* Filters */}
      <motion.div variants={fadeUp}>
        <Card className="mb-6">
        <CardContent className="pt-6">
          <div className="flex gap-4 items-end">
            <div className="flex-1">
              <label className="text-sm font-medium mb-2 block">Agent ID</label>
              <input
                type="text"
                className="w-full px-3 py-2 border rounded-md bg-background"
                placeholder="Filter by agent ID..."
                value={agentIdFilter}
                onChange={(e) => {
                  setAgentIdFilter(e.target.value);
                  setCurrentPage(0); // Reset to first page
                }}
              />
            </div>
            <div className="w-48">
              <label className="text-sm font-medium mb-2 block">Decision</label>
              <select
                className="w-full px-3 py-2 border rounded-md bg-background"
                value={decisionFilter ?? ''}
                onChange={(e) => {
                  setDecisionFilter(e.target.value ? parseInt(e.target.value) as 0 | 1 : undefined);
                  setCurrentPage(0); // Reset to first page
                }}
              >
                <option value="">All decisions</option>
                <option value="1">Allowed</option>
                <option value="0">Blocked</option>
              </select>
            </div>
            <Button
              variant="outline"
              onClick={loadSessions}
              disabled={isLoading}
            >
              {isLoading ? 'Loading...' : 'Refresh'}
            </Button>
          </div>
        </CardContent>
      </Card>
      </motion.div>

      {/* Error State */}
      {error && (
        <Card className="mb-6 border-red-500">
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-red-600 font-medium">Error loading sessions</p>
                <p className="text-sm text-muted-foreground mt-1">{error}</p>
              </div>
              <Button onClick={loadSessions} variant="outline">
                Retry
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Sessions Table */}
      <motion.div variants={scaleReveal}>
        <GradientCard variant="gradient">
          <CardHeader>
            <div className="flex justify-between items-center">
              <CardTitle className="text-2xl font-semibold">
                Enforcement Sessions
                {!isLoading && (
                  <span className="text-sm font-normal text-muted-foreground ml-2">
                    ({totalCount} total)
                  </span>
                )}
              </CardTitle>
              {isLoading && (
                <span className="text-sm text-muted-foreground">Loading...</span>
              )}
            </div>
          </CardHeader>
        <CardContent>
          {sessions.length === 0 && !isLoading ? (
            <div className="text-center py-12 text-muted-foreground">
              <p>No enforcement sessions found</p>
              <p className="text-sm mt-2">
                {agentIdFilter || decisionFilter !== undefined
                  ? 'Try adjusting your filters'
                  : 'Sessions will appear here when agents make enforcement calls'}
              </p>
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="font-mono">Session ID</TableHead>
                    <TableHead>Agent ID</TableHead>
                    <TableHead>Layer</TableHead>
                    <TableHead>Decision</TableHead>
                    <TableHead>Intent</TableHead>
                    <TableHead>Started</TableHead>
                    <TableHead>Duration</TableHead>
                    <TableHead>Rules</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sessions.map((session) => (
                    <TableRow
                      key={session.session_id}
                      className="cursor-pointer hover:bg-neutral-800/50 transition-colors"
                      onClick={() => navigate(`/console/agents/${session.session_id}`)}
                    >
                      <TableCell className="font-mono text-sm">
                        {session.session_id.substring(0, 12)}...
                      </TableCell>
                      <TableCell className="font-medium">{session.agent_id}</TableCell>
                      <TableCell>
                        <Badge variant="outline">{session.layer}</Badge>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary" className={getDecisionColor(session.final_decision)}>
                          {getDecisionLabel(session.final_decision)}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground max-w-xs truncate">
                        {session.intent_summary}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-sm">
                        {formatRelativeTime(session.timestamp_ms)}
                      </TableCell>
                      <TableCell className="text-sm">
                        {formatDuration(session.duration_us)}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{session.rules_evaluated_count}</Badge>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>

              {/* Pagination */}
              {totalPages > 1 && (
                <div className="flex items-center justify-between mt-4 pt-4 border-t">
                  <div className="text-sm text-muted-foreground">
                    Page {currentPage + 1} of {totalPages}
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setCurrentPage(p => Math.max(0, p - 1))}
                      disabled={currentPage === 0 || isLoading}
                    >
                      Previous
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setCurrentPage(p => Math.min(totalPages - 1, p + 1))}
                      disabled={currentPage >= totalPages - 1 || isLoading}
                    >
                      Next
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </GradientCard>
      </motion.div>
    </motion.div>
  );
};

export default AgentsIndexPage;
