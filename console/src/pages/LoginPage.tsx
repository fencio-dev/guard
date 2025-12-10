import { GradientCard, CardHeader, CardTitle, CardContent, CardDescription } from "@/components/ui/gradient-card";
import { AlertCircle } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { motion } from "framer-motion";
import { scaleReveal } from "@/lib/animations";

const LoginPage = () => {
  return (
    <motion.div
      className="flex h-screen items-center justify-center bg-neutral-950"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.5 }}
      style={{
        background: "radial-gradient(circle at center, rgb(23 23 23 / 0.5) 0%, rgb(10 10 10) 100%)"
      }}
    >
      <div className="w-full max-w-2xl p-8">
        <motion.div
          variants={scaleReveal}
          initial="hidden"
          animate="show"
        >
          <GradientCard variant="glass">
            <CardHeader>
              <CardTitle className="text-5xl font-bold bg-gradient-hero bg-clip-text text-transparent">
                Welcome to Fencio Guard
              </CardTitle>
              <CardDescription className="text-neutral-400 text-base mt-2">
                Manage your AI agent policies and security settings.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Alert>
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>
                  Please login via the{' '}
                  <a
                    href="https://developer.fencio.dev"
                    className="underline font-medium"
                  >
                    Fencio Developer Platform
                  </a>
                  {' '}to access this application.
                </AlertDescription>
              </Alert>

              <div className="text-sm text-muted-foreground">
                <p>
                  After logging in to the developer platform, you'll be redirected back here
                  automatically.
                </p>
              </div>
            </CardContent>
          </GradientCard>
        </motion.div>
      </div>
    </motion.div>
  );
};

export default LoginPage;
