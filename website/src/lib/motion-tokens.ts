/** Shared motion presets for docs chrome (springs, durations, easings). */
export const motionTokens = {
  duration: {
    instant: 0.12,
    fast: 0.16,
    exit: 0.18,
    standard: 0.24,
    considered: 0.48,
  },
  ease: {
    enter: [0.16, 1, 0.3, 1] as const,
    exit: [0.7, 0, 0.84, 0] as const,
    standard: [0.22, 1, 0.36, 1] as const,
    inOut: [0.65, 0, 0.35, 1] as const,
  },
  spring: {
    responsive: { type: "spring" as const, stiffness: 520, damping: 44 },
    gentle: { type: "spring" as const, stiffness: 340, damping: 34 },
    /** Presses and small indicators — settles fast, no overshoot. */
    snappy: { type: "spring" as const, visualDuration: 0.26, bounce: 0 },
    /** Panels and layout shifts. */
    smooth: { type: "spring" as const, visualDuration: 0.4, bounce: 0 },
    /** Width / shape morphs that follow new content. */
    morph: { type: "spring" as const, visualDuration: 0.42, bounce: 0 },
  },
} as const;
