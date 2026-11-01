// Hex pairs are the primary fill on light and dark surfaces, respectively.
export const accents = {
  neutral: ["#000000", "#ffffff"],
  blue: ["#245bc4", "#8bb4ff"],
  violet: ["#7142b6", "#bc9cff"],
  teal: ["#08776d", "#68d4c4"],
  rose: ["#b93c67", "#ff94b3"],
  amber: ["#955a08", "#f4be69"],
} as const;

export type AccentPreset = keyof typeof accents;
export type Accent = AccentPreset | `#${string}`;

export function normalizeHex(value: unknown): `#${string}` | undefined {
  if (typeof value !== "string") return;
  const digits = value.trim().replace(/^#/, "").toLowerCase();
  if (!/^(?:[0-9a-f]{3}|[0-9a-f]{6})$/.test(digits)) return;
  return `#${digits.length === 3 ? [...digits].map(d => d + d).join("") : digits}`;
}

export function isAccentPreset(value: unknown): value is AccentPreset {
  return typeof value === "string" && Object.hasOwn(accents, value);
}
export function isAccent(value: unknown): value is Accent {
  return isAccentPreset(value) || (typeof value === "string" && /^#[0-9a-f]{6}$/.test(value));
}

// Emit these before first paint so restored choices work on direct links too.
export const accentCss = Object.entries(accents).map(([id, [light, dark]]) => `
  :root,[data-kui-surface="light"] { --swatch-${id}: ${light}; }
  html[data-theme="dark"],[data-kui-surface="dark"] { --swatch-${id}: ${dark}; }
  html[data-accent="${id}"] { --accent-light: ${light}; --accent-dark: ${dark}; --accent-wash: ${id === "neutral" ? 0 : 16}%; }
`).join("") + `
  html[data-accent^="#"] {
    --accent-light: var(--custom-accent);
    --accent-dark: var(--custom-accent);
    --accent-wash: 16%;
    --accent-text-light: color-mix(in srgb, var(--accent-light) 43%, black);
    --accent-text-dark: color-mix(in srgb, var(--accent-dark) 36%, white);
  }
`;
