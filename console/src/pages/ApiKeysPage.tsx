import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { GradientCard, CardHeader, CardTitle, CardContent } from "@/components/ui/gradient-card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gatewayApi } from "@/lib/gateway-api";
import { useAuth } from "@/contexts/AuthContext";
import type { ApiKey } from "@/types";
import { Key, Copy, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { motion } from "framer-motion";
import { staggerContainer, fadeUp, scaleReveal } from "@/lib/animations";

const ApiKeysPage = () => {
  const { session } = useAuth();
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [loading, setLoading] = useState(true);
  const [showGenerateDialog, setShowGenerateDialog] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  const [generatedKey, setGeneratedKey] = useState("");
  const [deleteKeyId, setDeleteKeyId] = useState<string | null>(null);

  useEffect(() => {
    if (session?.access_token) {
      fetchKeys();
    }
  }, [session]);

  const fetchKeys = async () => {
    if (!session?.access_token) return;
    try {
      setLoading(true);
      const fetchedKeys = await gatewayApi.listApiKeys(session.access_token);
      setKeys(fetchedKeys);
    } catch (error) {
      console.error("Failed to fetch keys:", error);
      toast.error("Failed to load API keys");
    } finally {
      setLoading(false);
    }
  };

  const handleGenerate = async () => {
    if (!newKeyName.trim()) {
      toast.error("Please enter a name for the API key");
      return;
    }
    
    if (!session?.access_token) return;

    try {
      const newKey = await gatewayApi.generateApiKey(session.access_token, newKeyName);
      
      // Update local list
      setKeys([newKey, ...keys]);
      setGeneratedKey(newKey.key || ""); // The API returns the full key on creation
      setNewKeyName("");
      toast.success("API key created successfully");
    } catch (error) {
      console.error("Failed to generate key:", error);
      toast.error("Failed to generate API key");
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(generatedKey);
    toast.success("API key copied to clipboard");
  };

  const handleDelete = async (keyString: string) => {
    if (!session?.access_token) return;
    
    try {
      await gatewayApi.revokeApiKey(session.access_token, keyString);
      setKeys(keys.filter(k => k.key !== keyString)); // assuming key property matches, usually we use ID but here key is ID
      setDeleteKeyId(null);
      toast.success("API key deleted");
    } catch (error) {
      console.error("Failed to delete key:", error);
      toast.error("Failed to delete API key");
    }
  };

  // const formatRelativeTime = (dateString: string | Date | null) => {
  //   if (!dateString) return 'Never';
  //   const date = new Date(dateString);
  //   const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  //   if (seconds < 60) return `${seconds}s ago`;
  //   if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  //   if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  //   return `${Math.floor(seconds / 86400)}d ago`;
  // };

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
        <h1 className="text-4xl font-bold bg-gradient-hero bg-clip-text text-transparent">API Keys</h1>
        <Button onClick={() => setShowGenerateDialog(true)} variant="primary-composio">
          <Key className="h-4 w-4 mr-2" />
          Generate API Key
        </Button>
      </motion.div>

      <motion.div variants={scaleReveal}>
        <GradientCard variant="gradient">
        <CardHeader>
          <CardTitle>Your API Keys</CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <p className="text-muted-foreground">Loading keys...</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead className="font-mono">Key Preview</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead>Last Used</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {keys.length === 0 ? (
                   <TableRow>
                    <TableCell colSpan={5} className="text-center text-muted-foreground">
                      No API keys found. Generate one to get started.
                    </TableCell>
                  </TableRow>
                ) : (
                  keys.map((key) => (
                    <TableRow key={key.key}> {/* Using key string as key for react list */}
                      <TableCell className="font-medium">{key.description || "Untitled"}</TableCell>
                      <TableCell className="font-mono text-sm text-muted-foreground">
                        {key.keyPrefix || "..."}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {new Date(key.createdAt).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {/* formatRelativeTime(key.lastUsed) - lastUsed not in simplified type yet */}
                        -
                      </TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => setDeleteKeyId(key.key)} // Pass key string (which serves as ID)
                        >
                          <Trash2 className="h-4 w-4 text-destructive" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </GradientCard>
      </motion.div>

      {/* Generate Key Dialog */}
      <Dialog open={showGenerateDialog} onOpenChange={setShowGenerateDialog}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {generatedKey ? 'API Key Generated' : 'Generate API Key'}
            </DialogTitle>
            <DialogDescription>
              {generatedKey
                ? 'Save this key securely. You won\'t be able to see it again.'
                : 'Create a new API key to access the MCP Gateway.'}
            </DialogDescription>
          </DialogHeader>

          {!generatedKey ? (
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Key Name</label>
                <Input
                  placeholder="e.g., Production Server"
                  value={newKeyName}
                  onChange={(e) => setNewKeyName(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleGenerate()}
                />
              </div>
            </div>
          ) : (
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Your API Key</label>
                <div className="flex gap-2">
                  <Input
                    readOnly
                    value={generatedKey}
                    className="font-mono text-sm"
                  />
                  <Button variant="outline" size="icon" onClick={handleCopy}>
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
              </div>
              <div className="bg-yellow-500/10 text-yellow-700 dark:text-yellow-400 p-3 rounded-md text-sm">
                Make sure to copy your API key now. You won't be able to see it again!
              </div>
            </div>
          )}

          <DialogFooter>
            {!generatedKey ? (
              <>
                <Button variant="outline" onClick={() => setShowGenerateDialog(false)}>
                  Cancel
                </Button>
                <Button onClick={handleGenerate}>Generate</Button>
              </>
            ) : (
              <Button onClick={() => {
                setShowGenerateDialog(false);
                setGeneratedKey("");
              }}>
                Done
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog open={!!deleteKeyId} onOpenChange={() => setDeleteKeyId(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Delete API Key</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this API key? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteKeyId(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => deleteKeyId && handleDelete(deleteKeyId)}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </motion.div>
  );
};

export default ApiKeysPage;