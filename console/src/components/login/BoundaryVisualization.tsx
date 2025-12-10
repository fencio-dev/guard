import { motion, AnimatePresence } from "framer-motion";

interface BoundaryVisualizationProps {
  stage: 0 | 1 | 2;
}

export function BoundaryVisualization({ stage }: BoundaryVisualizationProps) {
  // SVG dimensions - compact and responsive
  const width = 500;
  const height = 300;
  const centerX = width / 2;
  const centerY = height / 2;

  // Pastel colors - mint, sky, soft purple
  const colors = {
    mint: "#A7F3D0",
    sky: "#BAE6FD",
    purple: "#DDD6FE",
    pastelRed: "#FCA5A5",
    intersectionGlow: "#E0F2FE", // Bright pastel for intersection
  };

  // Single policy example (no shuffle)
  const policyExample = "Don't perform write operations on production database";

  // Three overlapping circles - compact sizing
  const boundaries = [
    { cx: centerX - 65, cy: centerY - 30, r: 80, color: colors.mint },
    { cx: centerX + 65, cy: centerY - 30, r: 80, color: colors.sky },
    { cx: centerX, cy: centerY + 40, r: 80, color: colors.purple },
  ];

  // Agent position (at intersection)
  const agentX = centerX;
  const agentY = centerY;

  // Action attempt position (closer, just outside boundaries)
  const attemptX = centerX + 105;
  const attemptY = centerY - 52;

  // Boundary animation variants - smooth and gentle
  const boundaryVariants = {
    idle: (color: string) => ({
      opacity: 0.25,
      stroke: color,
      strokeWidth: 1.5,
      fillOpacity: 0.12,
    }),
    checking: (color: string) => ({
      opacity: [0.25, 0.4, 0.3],
      stroke: color,
      strokeWidth: [1.5, 2, 1.5],
      fillOpacity: [0.12, 0.18, 0.15],
      transition: {
        duration: 2.5,
        repeat: Infinity,
        ease: "easeInOut" as const,
      },
    }),
    blocked: {
      opacity: [0.3, 0.5, 0.35],
      stroke: colors.pastelRed,
      strokeWidth: [1.5, 2.5, 2],
      fillOpacity: [0.15, 0.25, 0.18],
      transition: {
        duration: 1,
        ease: "easeInOut" as const,
      },
    },
  };

  const stageAnimation = stage === 0 ? "idle" : stage === 1 ? "checking" : "blocked";

  return (
    <div className="w-full">
      <div className="rounded-lg border border-border bg-card p-4 md:p-6 shadow-md">
        {/* Header */}
        <div className="mb-4">
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Design Boundaries
          </span>
        </div>

        {/* SVG Canvas - compact and responsive */}
        <div className="flex items-center justify-center">
          <svg
            viewBox={`0 0 ${width} ${height}`}
            className="w-full h-auto"
            style={{ maxWidth: "600px", maxHeight: "360px" }}
          >
            {/* Definitions */}
            <defs>
              {/* Background Grid (dots) */}
              <pattern
                id="dot-grid"
                width="20"
                height="20"
                patternUnits="userSpaceOnUse"
              >
                <circle cx="2" cy="2" r="0.8" fill="hsl(214, 32%, 91%)" opacity="0.25" />
              </pattern>

              {/* Intersection radial glow - CRITICAL HIGHLIGHT */}
              <radialGradient id="intersection-glow" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stopColor={colors.intersectionGlow} stopOpacity="0.9" />
                <stop offset="50%" stopColor={colors.mint} stopOpacity="0.6" />
                <stop offset="100%" stopColor={colors.sky} stopOpacity="0.3" />
              </radialGradient>

              {/* Dot pattern for intersection */}
              <pattern
                id="intersection-dots"
                width="8"
                height="8"
                patternUnits="userSpaceOnUse"
              >
                <circle cx="4" cy="4" r="1" fill={colors.sky} opacity="0.4" />
              </pattern>
            </defs>
            <rect width={width} height={height} fill="url(#dot-grid)" />

            {/* CRITICAL: Enhanced Intersection Highlight - compact size */}
            {/* Layer 1: Radial glow */}
            <motion.circle
              cx={agentX}
              cy={agentY}
              r="44"
              fill="url(#intersection-glow)"
              opacity={stage === 2 ? 0.7 : 0.85}
              animate={
                stage === 0
                  ? {
                      opacity: [0.85, 0.95, 0.85],
                      r: [44, 47, 44],
                      transition: { duration: 3, repeat: Infinity, ease: "easeInOut" as const },
                    }
                  : stage === 1
                  ? {
                      opacity: [0.85, 1, 0.85],
                      r: [44, 48, 44],
                      transition: { duration: 2, repeat: Infinity, ease: "easeInOut" as const },
                    }
                  : undefined
              }
            />

            {/* Layer 2: Dot pattern overlay */}
            <circle
              cx={agentX}
              cy={agentY}
              r="40"
              fill="url(#intersection-dots)"
              opacity={0.6}
            />

            {/* Layer 3: Soft border around intersection */}
            <motion.circle
              cx={agentX}
              cy={agentY}
              r="40"
              fill="none"
              stroke={colors.sky}
              strokeWidth="1.8"
              opacity={stage === 2 ? 0.3 : 0.5}
              strokeDasharray="5,5"
            />

            {/* Design Boundary Circles */}
            {boundaries.map((boundary, i) => (
              <g key={i}>
                <motion.circle
                  cx={boundary.cx}
                  cy={boundary.cy}
                  r={boundary.r}
                  fill={stage === 2 ? colors.pastelRed : boundary.color}
                  stroke={stage === 2 ? colors.pastelRed : boundary.color}
                  custom={boundary.color}
                  variants={boundaryVariants}
                  initial="idle"
                  animate={stageAnimation}
                />
                {/* Subtle label for each guardrail */}
                <text
                  x={boundary.cx}
                  y={i === 2 ? boundary.cy + boundary.r + 18 : boundary.cy - boundary.r - 8}
                  textAnchor="middle"
                  className="fill-muted-foreground text-[9px] font-normal"
                  opacity="0.5"
                >
                  Guardrail {i + 1}
                </text>
              </g>
            ))}

            {/* Policy Example - Only show in Stage 0 with smooth fade */}
            <AnimatePresence>
              {stage === 0 && (
                <motion.foreignObject
                  x={boundaries[0].cx - 120}
                  y={boundaries[0].cy + boundaries[0].r + 10}
                  width="240"
                  height="40"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{
                    duration: 1,
                    ease: "easeInOut" as const,
                  }}
                >
                  <div
                    style={{
                      fontSize: "10px",
                      fontStyle: "italic",
                      color: "hsl(215, 16%, 47%)",
                      textAlign: "center",
                      lineHeight: "1.3",
                      padding: "0 8px",
                    }}
                  >
                    "{policyExample}"
                  </div>
                </motion.foreignObject>
              )}
            </AnimatePresence>

            {/* Agent Text (free, no encapsulation) */}
            <motion.text
              x={agentX}
              y={agentY + 5}
              textAnchor="middle"
              className="fill-primary text-[12px] font-medium"
              animate={
                stage === 0
                  ? {
                      opacity: [0.8, 1, 0.8],
                      transition: { duration: 3, repeat: Infinity, ease: "easeInOut" as const },
                    }
                  : stage === 1
                  ? {
                      opacity: [0.9, 1, 0.9],
                      scale: [1, 1.05, 1],
                      transition: { duration: 2, repeat: Infinity, ease: "easeInOut" as const },
                    }
                  : { opacity: 0.7, scale: 0.95 }
              }
            >
              AI Agent
            </motion.text>

            {/* Stage 1: Checking animation - smooth expanding ripple */}
            {stage === 1 && (
              <>
                <motion.circle
                  cx={agentX}
                  cy={agentY}
                  r="16"
                  fill="none"
                  stroke={colors.sky}
                  strokeWidth="2"
                  initial={{ r: 16, opacity: 0.5 }}
                  animate={{
                    r: [16, 64, 80],
                    opacity: [0.5, 0.2, 0],
                  }}
                  transition={{
                    duration: 2.5,
                    repeat: Infinity,
                    ease: "easeOut" as const,
                  }}
                />
              </>
            )}

            {/* Stage 2: Blocked - show attempted action outside boundaries */}
            {stage === 2 && (
              <>
                {/* Line from agent to attempted action */}
                <motion.line
                  x1={agentX}
                  y1={agentY}
                  x2={attemptX}
                  y2={attemptY}
                  stroke={colors.pastelRed}
                  strokeWidth="2"
                  strokeDasharray="6,6"
                  initial={{ pathLength: 0, opacity: 0 }}
                  animate={{ pathLength: 1, opacity: 0.5 }}
                  transition={{ duration: 0.6, ease: "easeOut" as const }}
                />

                {/* Cross/X icon on the line (midpoint) */}
                <motion.g
                  initial={{ opacity: 0, scale: 0 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ duration: 0.3, delay: 0.4 }}
                >
                  {/* Calculate midpoint of the line */}
                  <circle
                    cx={(agentX + attemptX) / 2}
                    cy={(agentY + attemptY) / 2}
                    r="10"
                    fill="white"
                    stroke={colors.pastelRed}
                    strokeWidth="2"
                  />
                  {/* X mark - two lines crossing */}
                  <line
                    x1={(agentX + attemptX) / 2 - 5}
                    y1={(agentY + attemptY) / 2 - 5}
                    x2={(agentX + attemptX) / 2 + 5}
                    y2={(agentY + attemptY) / 2 + 5}
                    stroke={colors.pastelRed}
                    strokeWidth="2"
                    strokeLinecap="round"
                  />
                  <line
                    x1={(agentX + attemptX) / 2 + 5}
                    y1={(agentY + attemptY) / 2 - 5}
                    x2={(agentX + attemptX) / 2 - 5}
                    y2={(agentY + attemptY) / 2 + 5}
                    stroke={colors.pastelRed}
                    strokeWidth="2"
                    strokeLinecap="round"
                  />
                </motion.g>

                {/* Attempted action point */}
                <motion.circle
                  cx={attemptX}
                  cy={attemptY}
                  r="8"
                  fill={colors.pastelRed}
                  initial={{ scale: 0, opacity: 0 }}
                  animate={{ scale: 1, opacity: 0.8 }}
                  transition={{ duration: 0.4, delay: 0.3 }}
                />

                {/* Label for attempted action */}
                <motion.text
                  x={attemptX}
                  y={attemptY - 25}
                  textAnchor="middle"
                  className="fill-red-600 text-[11px] font-medium"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ delay: 0.5 }}
                >
                  Attempting harmful action
                </motion.text>
                <motion.text
                  x={attemptX}
                  y={attemptY - 12}
                  textAnchor="middle"
                  className="fill-red-600 text-[11px] font-medium"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ delay: 0.5 }}
                >
                  outside design boundary
                </motion.text>

                {/* Soft pulse from agent */}
                <motion.circle
                  cx={agentX}
                  cy={agentY}
                  r="16"
                  fill={colors.pastelRed}
                  initial={{ r: 16, opacity: 0.4 }}
                  animate={{
                    r: [16, 96, 112],
                    opacity: [0.4, 0.15, 0],
                  }}
                  transition={{ duration: 1.2, ease: "easeOut" as const }}
                />
              </>
            )}
          </svg>
        </div>
      </div>
    </div>
  );
}
