import { motion } from "framer-motion";
import { useAuth } from "../../contexts/AuthContext";
import { Button } from "../ui/button";

const containerVariants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: {
      staggerChildren: 0.1,
      delayChildren: 0.2,
    },
  },
};

const itemVariants = {
  hidden: { opacity: 0, y: 10 },
  visible: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.4, ease: "easeOut" as const },
  },
};

export function LoginPanel() {
  const { signIn } = useAuth();

  const handleSignIn = async () => {
    try {
      await signIn();
    } catch (error) {
      console.error("Sign in error:", error);
    }
  };

  return (
    <div className="flex h-full w-full flex-col items-center justify-center bg-background p-6 md:p-12">
      <motion.div
        className="flex max-w-md flex-col items-center text-center"
        variants={containerVariants}
        initial="hidden"
        animate="visible"
      >
        {/* Logo/Branding */}
        <motion.div variants={itemVariants} className="mb-12">
          <h1 className="text-4xl font-bold">
            <span className="text-foreground">Welcome</span>
            <span className="text-primary">.</span>
          </h1>
        </motion.div>

        {/* Welcome Content */}
        {/* Don't want to add Tupl branding right now. Re-branding is coming soon */}
        {/* <motion.h2
          variants={itemVariants}
          className="mb-4 text-3xl font-semibold text-foreground"
        >
          Welcome
        </motion.h2> */}

        <motion.p
          variants={itemVariants}
          className="mb-2 text-muted-foreground"
        >
          Secure your AI agents with intelligent, operational guardrails
        </motion.p>

        <motion.p
          variants={itemVariants}
          className="mb-8 text-muted-foreground"
        >
          And save tokens while doing it.
        </motion.p>

        {/* Google OAuth Button */}
        <motion.div variants={itemVariants} className="w-full">
          <Button
            onClick={handleSignIn}
            className="w-full gap-3 shadow-sm"
            size="lg"
          >
            <svg className="h-5 w-5" viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
              />
              <path
                fill="currentColor"
                d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
              />
              <path
                fill="currentColor"
                d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
              />
              <path
                fill="currentColor"
                d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
              />
            </svg>
            Sign in with Google
          </Button>
        </motion.div>

        {/* Footer */}
        <motion.div variants={itemVariants} className="mt-12">
          <p className="text-xs text-muted-foreground">
            Need help?{" "}
            <a
              href="mailto:support@tupl.com"
              className="text-primary hover:underline"
            >
              core@elitrotechnologies.com
            </a>
          </p>
        </motion.div>
      </motion.div>
    </div>
  );
}
