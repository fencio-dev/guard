import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { controlPlaneApi } from '@/lib/control-plane-api';
import type { RuleConfigResponse } from '@/types';
import { Shield, Plus, Trash2, Edit, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';

export function PoliciesPage() {
  const [configs, setConfigs] = useState<RuleConfigResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadConfigs();
  }, []);

  const loadConfigs = async () => {
    try {
      setLoading(true);
      const response = await controlPlaneApi.listConfigs();
      setConfigs(response?.configurations || []);
      setError(null);
    } catch (err) {
      console.error('Failed to load policies:', err);
      setError('Failed to load policy configurations. Please ensure the Control Plane is running.');
    } finally {
      setLoading(false);
    }
  };

  const deleteConfig = async (agentId: string) => {
    if (!confirm('Are you sure you want to delete this policy? This will remove all rules for the agent.')) {
      return;
    }

    try {
      await controlPlaneApi.deleteConfig(agentId);
      await loadConfigs();
    } catch (err) {
      console.error('Failed to delete policy:', err);
      alert('Failed to delete policy');
    }
  };

  if (loading) {
    return <div className="p-8 text-center">Loading policies...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">Security Policies</h2>
          <p className="text-muted-foreground">
            Manage agent profiles and security boundaries.
          </p>
        </div>
        <Button asChild>
          <Link to="/console/policies/new">
            <Plus className="mr-2 h-4 w-4" />
            New Policy
          </Link>
        </Button>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Error</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="grid gap-6">
        {(configs?.length || 0) === 0 && !error ? (
          <Card>
            <CardContent className="py-8 text-center text-muted-foreground">
              No policies configured yet. Click "New Policy" to get started.
            </CardContent>
          </Card>
        ) : (
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Agent ID</TableHead>
                  <TableHead>Owner</TableHead>
                  <TableHead>Active Rules</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {configs.map((config) => (
                  <TableRow key={config.agent_id}>
                    <TableCell className="font-medium flex items-center gap-2">
                      <Shield className="h-4 w-4 text-green-600" />
                      {config.agent_id}
                    </TableCell>
                    <TableCell>{config.owner}</TableCell>
                    <TableCell>
                      <div className="flex gap-2">
                        {Object.entries(config.rules_by_layer).map(([layer, count]) => (
                          <span key={layer} className="inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80">
                            {layer}: {count}
                          </span>
                        ))}
                        {config.rule_count === 0 && <span className="text-muted-foreground text-sm">None</span>}
                      </div>
                    </TableCell>
                    <TableCell>{new Date(config.created_at).toLocaleDateString()}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-2">
                        <Button variant="ghost" size="icon" asChild>
                          <Link to={`/console/policies/${config.agent_id}`}>
                            <Edit className="h-4 w-4" />
                          </Link>
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="text-destructive hover:text-destructive/90"
                          onClick={() => deleteConfig(config.agent_id)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </div>
  );
}
