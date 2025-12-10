import { useEffect, useState } from "react";
import { CodeBlock } from "./CodeBlock";
import { BoundaryVisualization } from "./BoundaryVisualization";

// Demo configuration
const DEMO_CODE = `agent.execute({
  "action": "delete_database",
  "resource": "production_db"
})`;

const STAGE_DURATIONS = [5000, 2000, 3000]; // [typing, checking, blocked] in ms

interface AnimatedDemoProps {
  autoplay?: boolean;
  loop?: boolean;
}

export function AnimatedDemo({ autoplay = true, loop = true }: AnimatedDemoProps) {
  const [currentStage, setCurrentStage] = useState<0 | 1 | 2>(0);
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  // Detect prefers-reduced-motion
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    setPrefersReducedMotion(mediaQuery.matches);

    const handleChange = (e: MediaQueryListEvent) => {
      setPrefersReducedMotion(e.matches);
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, []);

  // Stage progression effect
  useEffect(() => {
    // If reduced motion is preferred, show final state only
    if (prefersReducedMotion) {
      setCurrentStage(2);
      return;
    }

    if (!autoplay) return;

    const duration = STAGE_DURATIONS[currentStage];
    const timer = setTimeout(() => {
      setCurrentStage((prev) => {
        const nextStage = prev + 1;
        if (nextStage > 2) {
          return loop ? 0 : 2; // Loop back to 0 or stay at 2
        }
        return nextStage as 0 | 1 | 2;
      });
    }, duration);

    return () => clearTimeout(timer);
  }, [currentStage, autoplay, loop, prefersReducedMotion]);

  return (
    <div
      className="flex h-full w-full flex-col items-center justify-center gap-8 bg-subtle-bg p-6 md:p-12"
      aria-label="Security policy demonstration"
    >
      {/* Code Block - 60% of space */}
      <div className="w-full max-w-2xl">
        <CodeBlock stage={currentStage} code={DEMO_CODE} language="python" />
      </div>

      {/* Boundary Visualization - 40% of space */}
      <div className="w-full max-w-2xl">
        <BoundaryVisualization stage={currentStage} />
      </div>
    </div>
  );
}
