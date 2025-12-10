import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { fetchSessionDetail, type SessionDetail } from "@/lib/telemetry-api";

const AgentDetailPage = () => {
  const { sessionId } = useParams<{ sessionId: string }>();
  const navigate = useNavigate();

  const [session, setSession] = useState<SessionDetail | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!sessionId) {
      setError('No session ID provided');
      setIsLoading(false);
      return;
    }

    const loadSession = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const data = await fetchSessionDetail(sessionId);
        setSession(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load session');
        console.error('Failed to fetch session:', err);
      } finally {
        setIsLoading(false);
      }
    };

    loadSession();
  }, [sessionId]);

  const getDecisionColor = (decision: 0 | 1) => {
    return decision === 1
      ? 'bg-green-500/10 text-green-700 dark:text-green-400 border-green-500/20'
      : 'bg-red-500/10 text-red-700 dark:text-red-400 border-red-500/20';
  };

  const getDecisionLabel = (decision: 0 | 1) => {
    return decision === 1 ? 'ALLOW' : 'BLOCK';
  };

  const formatDuration = (durationUs: number) => {
    const ms = durationUs / 1000;
    if (ms < 1000) return `${ms.toFixed(2)}ms`;
    return `${(ms / 1000).toFixed(3)}s`;
  };

  const formatTimestamp = (timestampMs: number) => {
    return new Date(timestampMs).toLocaleString();
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-96">
        <div className="text-center">
          <div className="text-lg font-medium mb-2">Loading session...</div>
          <div className="text-sm text-muted-foreground">Fetching enforcement details</div>
        </div>
      </div>
    );
  }

  // Error state
  if (error || !session) {
    return (
      <div className="flex items-center justify-center min-h-96">
        <Card className="max-w-md border-red-500">
          <CardContent className="pt-6">
            <div className="text-center">
              <h2 className="text-lg font-semibold text-red-600 mb-2">
                {error?.includes('not found') ? 'Session Not Found' : 'Error Loading Session'}
              </h2>
              <p className="text-sm text-muted-foreground mb-4">{error}</p>
              <div className="flex gap-2 justify-center">
                <Button onClick={() => navigate('/agents')} variant="outline">
                  Back to Sessions
                </Button>
                <Button onClick={() => window.location.reload()}>
                  Retry
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => navigate('/console/agents')}
            className="mb-2"
          >
            ← Back to Sessions
          </Button>
          <h1 className="text-2xl font-semibold">Enforcement Session Detail</h1>
          <div className="flex items-center gap-2 mt-2">
            <code className="text-sm bg-muted px-2 py-1 rounded">{session.session.session_id}</code>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => copyToClipboard(session.session.session_id)}
            >
              Copy ID
            </Button>
          </div>
        </div>
        <Badge variant="secondary" className={`${getDecisionColor(session.session.final_decision)} text-lg px-4 py-2`}>
          {getDecisionLabel(session.session.final_decision)}
        </Badge>
      </div>

      {/* Metadata Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium text-muted-foreground">Agent ID</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{session.session.agent_id}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium text-muted-foreground">Tenant ID</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">{session.session.tenant_id}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium text-muted-foreground">Layer</CardTitle>
          </CardHeader>
          <CardContent>
            <Badge variant="outline" className="text-base">{session.session.layer}</Badge>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium text-muted-foreground">Timestamp</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-sm">{formatTimestamp(session.session.timestamp_ms)}</div>
          </CardContent>
        </Card>
      </div>

      {/* Performance Metrics */}
      {session.session.performance && (
        <Card>
          <CardHeader>
            <CardTitle>Performance Metrics</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-6">
              <div>
                <div className="text-sm text-muted-foreground mb-1">Encoding Duration</div>
                <div className="text-2xl font-semibold">{formatDuration(session.session.performance.encoding_duration_us)}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground mb-1">Rule Query Duration</div>
                <div className="text-2xl font-semibold">{formatDuration(session.session.performance.rule_query_duration_us)}</div>
              </div>
              <div>
                <div className="text-sm text-muted-foreground mb-1">Evaluation Duration</div>
                <div className="text-2xl font-semibold">{formatDuration(session.session.performance.evaluation_duration_us)}</div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Intent Event */}
      <Card>
        <CardHeader>
          <CardTitle>Intent Event</CardTitle>
        </CardHeader>
        <CardContent>
          {session.session.intent && Object.keys(session.session.intent).length > 0 ? (
            <pre className="bg-neutral-900 border border-neutral-800 p-4 rounded-lg overflow-auto max-h-96 text-sm font-mono">
              {JSON.stringify(session.session.intent, null, 2)}
            </pre>
          ) : (
            <div className="bg-neutral-900 border border-neutral-800 p-6 rounded-lg text-center text-muted-foreground">
              No intent event data available
            </div>
          )}
        </CardContent>
      </Card>

      {/* Rules Evaluated */}
      <Card>
        <CardHeader>
          <CardTitle>Rules Evaluated ({session.session.rules_evaluated.length})</CardTitle>
        </CardHeader>
        <CardContent>
          {session.session.rules_evaluated.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No rules were evaluated for this session
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Rule ID</TableHead>
                  <TableHead>Family</TableHead>
                  <TableHead>Decision</TableHead>
                  <TableHead>Slice Similarities</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {session.session.rules_evaluated.map((rule, i) => (
                  <TableRow key={i}>
                    <TableCell className="font-mono text-sm">{rule.rule_id}</TableCell>
                    <TableCell>{rule.rule_family}</TableCell>
                    <TableCell>
                      <Badge
                        variant="secondary"
                        className={getDecisionColor(rule.decision as 0 | 1)}
                      >
                        {getDecisionLabel(rule.decision as 0 | 1)}
                      </Badge>
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {rule.slice_similarities.length > 0
                        ? rule.slice_similarities.map((s: number) => s.toFixed(3)).join(', ')
                        : 'N/A'}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Execution Timeline */}
      {session.session.events && session.session.events.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Execution Timeline ({session.session.events.length} events)</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {session.session.events.map((event, i) => (
                <div key={i} className="flex gap-4 items-start border-l-2 border-muted pl-4 py-2">
                  <div className="text-sm text-muted-foreground font-mono min-w-32">
                    {formatDuration(event.timestamp_us)}
                  </div>
                  <div className="flex-1">
                    <div className="font-medium">{event.type}</div>
                    {Object.keys(event).filter(k => k !== 'type' && k !== 'timestamp_us').length > 0 && (
                      <pre className="text-xs text-muted-foreground mt-1 bg-muted/50 p-2 rounded">
                        {JSON.stringify(
                          Object.fromEntries(
                            Object.entries(event).filter(([k]) => k !== 'type' && k !== 'timestamp_us')
                          ),
                          null,
                          2
                        )}
                      </pre>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
};

export default AgentDetailPage;

