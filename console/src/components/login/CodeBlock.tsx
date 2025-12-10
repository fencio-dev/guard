import { motion, AnimatePresence } from "framer-motion";
import { useEffect, useState } from "react";
import { AlertCircle, X } from "lucide-react";

interface CodeBlockProps {
  stage: 0 | 1 | 2;
  code: string;
  language?: string;
}

export function CodeBlock({ stage, code }: CodeBlockProps) {
  const [displayedCode, setDisplayedCode] = useState("");
  const [isTyping, setIsTyping] = useState(true);

  // Typing animation effect for stage 0
  useEffect(() => {
    if (stage === 0) {
      setDisplayedCode("");
      setIsTyping(true);
      let currentIndex = 0;
      const typingInterval = setInterval(() => {
        if (currentIndex <= code.length) {
          setDisplayedCode(code.slice(0, currentIndex));
          currentIndex++;
        } else {
          setIsTyping(false);
          clearInterval(typingInterval);
        }
      }, 30); // 30ms per character

      return () => clearInterval(typingInterval);
    } else {
      setDisplayedCode(code);
      setIsTyping(false);
    }
  }, [stage, code]);

  // Simple syntax highlighting
  const highlightSyntax = (text: string) => {
    return text
      .replace(
        /\b(agent|execute|delete_database|production_db)\b/g,
        '<span class="text-primary font-medium">$1</span>'
      )
      .replace(
        /"(action|resource)"/g,
        '<span class="text-purple-600">"$1"</span>'
      )
      .replace(
        /"(delete_database|production_db)"/g,
        stage === 1
          ? '<span class="text-orange-600 font-semibold underline decoration-red-500 decoration-2">"$1"</span>'
          : '<span class="text-orange-600">"$1"</span>'
      );
  };

  return (
    <div className="relative w-full">
      <div className="rounded-lg border border-border bg-card p-6 shadow-md">
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Agent Code
          </span>
          {stage === 1 && (
            <motion.div
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              className="flex items-center gap-1 text-xs text-orange-600"
            >
              <AlertCircle className="h-3 w-3" />
              <span>Analyzing...</span>
            </motion.div>
          )}
        </div>

        {/* Code Display */}
        <div className="relative">
          <pre className="font-mono text-sm leading-relaxed">
            <code
              dangerouslySetInnerHTML={{
                __html: highlightSyntax(displayedCode),
              }}
            />
            {isTyping && (
              <motion.span
                animate={{ opacity: [1, 0, 1] }}
                transition={{ duration: 0.8, repeat: Infinity }}
                className="inline-block h-5 w-2 bg-primary"
              />
            )}
          </pre>

          {/* Stage 1: Pulse animation on dangerous code */}
          {stage === 1 && (
            <motion.div
              className="absolute inset-0 rounded bg-orange-500/5"
              animate={{ opacity: [0.3, 0.5, 0.3] }}
              transition={{ duration: 1.5, repeat: Infinity }}
            />
          )}
        </div>
      </div>

      {/* Stage 2: Blocked Overlay */}
      <AnimatePresence>
        {stage === 2 && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.3 }}
            className="absolute inset-0 flex items-center justify-center rounded-lg bg-red-500/10 backdrop-blur-[2px]"
          >
            <motion.div
              initial={{ scale: 0.8, y: 10 }}
              animate={{ scale: 1, y: 0 }}
              className="flex items-center gap-2 rounded-md bg-card px-4 py-2 shadow-lg"
            >
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-red-100">
                <X className="h-5 w-5 text-red-700" />
              </div>
              <span className="font-medium text-red-700">Blocked by policy</span>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
