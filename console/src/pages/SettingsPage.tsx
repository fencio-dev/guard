import { GradientCard, CardHeader, CardTitle, CardContent, CardDescription } from "@/components/ui/gradient-card";
import { useAuth } from "@/contexts/AuthContext";
import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { motion } from "framer-motion";
import { staggerContainer, fadeUp, scaleReveal } from "@/lib/animations";

export const SettingsPage = () => {
  const { user, apiKey } = useAuth();

  return (
    <motion.div
      className="space-y-6"
      variants={staggerContainer}
      initial="hidden"
      animate="show"
    >
      <motion.div variants={fadeUp}>
        <h1 className="text-4xl font-bold bg-gradient-hero bg-clip-text text-transparent">Settings</h1>
        <p className="text-base text-neutral-400 mt-2">
          Manage your account settings and authentication.
        </p>
      </motion.div>

      <motion.div variants={scaleReveal}>
        <GradientCard variant="gradient">
        <CardHeader>
          <CardTitle>Authentication</CardTitle>
          <CardDescription>
            Your authentication is managed by the Fencio Developer Platform.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <div className="text-sm font-medium">User ID</div>
            <div className="font-mono text-sm bg-muted p-3 rounded-md">
              {user?.id || 'Not authenticated'}
            </div>
          </div>

          {apiKey && (
            <div className="space-y-2">
              <div className="text-sm font-medium">API Key (from Developer Platform)</div>
              <div className="font-mono text-sm bg-muted p-3 rounded-md truncate">
                {apiKey}
              </div>
            </div>
          )}

          <Alert>
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>
              To manage your API keys and authentication settings, visit the{' '}
              <a
                href="https://developer.fencio.dev"
                className="underline font-medium"
                target="_blank"
                rel="noopener noreferrer"
              >
                Fencio Developer Platform
              </a>
              .
            </AlertDescription>
          </Alert>
        </CardContent>
      </GradientCard>
      </motion.div>
    </motion.div>
  );
};
