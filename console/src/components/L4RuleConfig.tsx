import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Plus, Trash2 } from 'lucide-react';
import type { AgentRuleFamilies, ToolParamConstraintConfig } from '@/types';

interface L4RuleConfigProps {
  ruleFamilies: AgentRuleFamilies;
  onChange: (ruleFamilies: AgentRuleFamilies) => void;
}

export function L4RuleConfig({ ruleFamilies, onChange }: L4RuleConfigProps) {
  const toolWhitelist = ruleFamilies.tool_whitelist || {
    enabled: false,
    params: { allowed_tool_ids: [], action: 'DENY' }
  };

  const paramConstraints = ruleFamilies.tool_param_constraints || [];

  const updateWhitelist = (updates: Partial<typeof toolWhitelist>) => {
    onChange({
      ...ruleFamilies,
      tool_whitelist: { ...toolWhitelist, ...updates }
    });
  };

  const updateWhitelistParams = (updates: Partial<typeof toolWhitelist.params>) => {
    updateWhitelist({
      params: { ...toolWhitelist.params, ...updates }
    });
  };

  const addAllowedTool = () => {
    updateWhitelistParams({
      allowed_tool_ids: [...toolWhitelist.params.allowed_tool_ids, '']
    });
  };

  const updateAllowedTool = (index: number, value: string) => {
    const newTools = [...toolWhitelist.params.allowed_tool_ids];
    newTools[index] = value;
    updateWhitelistParams({ allowed_tool_ids: newTools });
  };

  const removeAllowedTool = (index: number) => {
    const newTools = [...toolWhitelist.params.allowed_tool_ids];
    newTools.splice(index, 1);
    updateWhitelistParams({ allowed_tool_ids: newTools });
  };

  const addParamConstraint = () => {
    const newConstraint: ToolParamConstraintConfig = {
      enabled: true,
      params: {
        param_name: '',
        param_type: 'string',
        enforcement_mode: 'HARD',
        allowed_methods: []
      }
    };
    onChange({
      ...ruleFamilies,
      tool_param_constraints: [...paramConstraints, newConstraint]
    });
  };

  const updateParamConstraint = (index: number, updates: Partial<ToolParamConstraintConfig['params']>) => {
    const newConstraints = [...paramConstraints];
    newConstraints[index] = {
      ...newConstraints[index],
      params: { ...newConstraints[index].params, ...updates }
    };
    onChange({
      ...ruleFamilies,
      tool_param_constraints: newConstraints
    });
  };

  const removeParamConstraint = (index: number) => {
    const newConstraints = [...paramConstraints];
    newConstraints.splice(index, 1);
    onChange({
      ...ruleFamilies,
      tool_param_constraints: newConstraints
    });
  };

  return (
    <div className="space-y-8">
      {/* Tool Whitelist Section */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="text-lg font-medium">Tool Whitelist</CardTitle>
            <Switch
              checked={toolWhitelist.enabled}
              onCheckedChange={(checked) => updateWhitelist({ enabled: checked })}
            />
          </div>
        </CardHeader>
        {toolWhitelist.enabled && (
          <CardContent className="space-y-4">
            <div className="grid gap-2">
              <Label>Default Action</Label>
              <Select
                value={toolWhitelist.params.action}
                onValueChange={(value: "ALLOW" | "DENY") => updateWhitelistParams({ action: value })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="DENY">Deny (Block unknown tools)</SelectItem>
                  <SelectItem value="ALLOW">Allow (Log unknown tools)</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label>Allowed Tool IDs</Label>
                <Button variant="outline" size="sm" onClick={addAllowedTool}>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Tool
                </Button>
              </div>
              {toolWhitelist.params.allowed_tool_ids.map((toolId, index) => (
                <div key={index} className="flex gap-2">
                  <Input
                    value={toolId}
                    onChange={(e) => updateAllowedTool(index, e.target.value)}
                    placeholder="e.g., web_search, calculator"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => removeAllowedTool(index)}
                    className="text-destructive hover:text-destructive/90"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              {toolWhitelist.params.allowed_tool_ids.length === 0 && (
                <p className="text-sm text-muted-foreground">No tools whitelisted yet.</p>
              )}
            </div>
          </CardContent>
        )}
      </Card>

      {/* Parameter Constraints Section */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle className="text-lg font-medium">Parameter Constraints</CardTitle>
            <Button variant="outline" size="sm" onClick={addParamConstraint}>
              <Plus className="h-4 w-4 mr-2" />
              Add Constraint
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-6">
          {paramConstraints.map((constraint, index) => (
            <div key={index} className="p-4 border rounded-lg space-y-4 bg-card/50">
              <div className="flex justify-between items-start">
                <h4 className="font-medium">Constraint #{index + 1}</h4>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeParamConstraint(index)}
                  className="text-destructive hover:text-destructive/90"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Tool ID (Optional)</Label>
                  <Input
                    value={constraint.params.tool_id || ''}
                    onChange={(e) => updateParamConstraint(index, { tool_id: e.target.value })}
                    placeholder="Apply to specific tool..."
                  />
                </div>
                <div className="space-y-2">
                  <Label>Parameter Name</Label>
                  <Input
                    value={constraint.params.param_name}
                    onChange={(e) => updateParamConstraint(index, { param_name: e.target.value })}
                    placeholder="*"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Type</Label>
                  <Select
                    value={constraint.params.param_type}
                    onValueChange={(value: any) => updateParamConstraint(index, { param_type: value })}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="string">String</SelectItem>
                      <SelectItem value="number">Number</SelectItem>
                      <SelectItem value="integer">Integer</SelectItem>
                      <SelectItem value="boolean">Boolean</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label>Enforcement Mode</Label>
                  <Select
                    value={constraint.params.enforcement_mode}
                    onValueChange={(value: any) => updateParamConstraint(index, { enforcement_mode: value })}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="HARD">Hard (Block)</SelectItem>
                      <SelectItem value="SOFT">Soft (Log Only)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="space-y-2">
                <Label>Regex Pattern (Optional)</Label>
                <Input
                  value={constraint.params.regex || ''}
                  onChange={(e) => updateParamConstraint(index, { regex: e.target.value })}
                  placeholder="e.g., ^[a-zA-Z0-9]+$"
                  className="font-mono"
                />
              </div>
            </div>
          ))}
          {paramConstraints.length === 0 && (
            <p className="text-sm text-muted-foreground">No parameter constraints defined.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
