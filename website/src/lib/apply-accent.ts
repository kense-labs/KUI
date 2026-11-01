import { accents, isAccent, isAccentPreset, normalizeHex, type Accent } from "./accent-palette";
import { previewRuntime } from "./preview-runtime";

const root = document.documentElement;
const motion = matchMedia("(prefers-reduced-motion: reduce)");
const storageKey = "kui-accent";
const customKey = "kui-custom-accent";
type Pair = [number, number];
const packed = (hex: string) => parseInt(hex.slice(1), 16) * 256 + 255;
function endpoint(id: Accent): Pair {
  if (id === "neutral") return [0, 0];
  if (isAccentPreset(id)) return [packed(accents[id][0]), packed(accents[id][1])];
  const color = packed(id);
  return [color, color];
}
let committed: Accent = isAccent(root.dataset.accent) ? root.dataset.accent : "neutral";
let showing = committed;
let previewing = false;
let custom = normalizeHex(committed) ?? accents.blue[0];
try { custom = normalizeHex(localStorage.getItem(customKey)) ?? custom; } catch { /* Memory-only storage. */ }
export function getAccent(): Accent {
  return committed;
}
export function getCustomAccent(): string {
  return custom;
}

function selectSurface(element: HTMLElement, accent: Accent) {
  element.dataset.accent = accent;
  if (isAccentPreset(accent)) {
    element.style.removeProperty("--custom-accent");
    element.style.removeProperty("--custom-ink");
  } else {
    element.style.setProperty("--custom-accent", accent);
    element.style.setProperty("--custom-ink", inkOn(accent));
  }
}

function inkOn(hex: string) {
  const [r, g, b] = [1, 3, 5].map(i => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4;
  });
  return .2126 * r + .7152 * g + .0722 * b > .179129 ? "#000000" : "#ffffff";
}

// Premultiplied interpolation lets a colored accent fade into each control's own neutral fill.
function mix(a: number, b: number, t: number) {
  const from = (a & 255) * (1 - t), to = (b & 255) * t;
  if (from + to === 0) return 0;
  const channel = (shift: number) => Math.round((((a >>> shift) & 255) * from + ((b >>> shift) & 255) * to) / (from + to));
  return channel(24) * 0x1000000 + channel(16) * 0x10000 + channel(8) * 256 + Math.round(from + to);
}
const opaque = (rgba: number, base: number) => {
  const result = mix(base * 256 + 255, (rgba >>> 8) * 256 + 255, (rgba & 255) / 255);
  return "#" + (result >>> 8).toString(16).padStart(6, "0");
};

let displayed = endpoint(getAccent());
let frame = 0;
function present(colors: Pair, settled: boolean) {
  displayed = colors;
  if (settled) {
    for (const name of ["--accent-light", "--accent-dark", "--accent-wash", "--ink-light", "--ink-dark"]) root.style.removeProperty(name);
  } else {
    const light = opaque(colors[0], 0), dark = opaque(colors[1], 0xffffff);
    root.style.setProperty("--accent-light", light);
    root.style.setProperty("--accent-dark", dark);
    root.style.setProperty("--ink-light", inkOn(light));
    root.style.setProperty("--ink-dark", inkOn(dark));
    root.style.setProperty("--accent-wash", (colors[0] & 255) * 16 / 255 + "%");
  }
  for (const el of document.querySelectorAll<HTMLIFrameElement>("iframe.rust-story")) {
    previewRuntime(el)?.accent?.(...colors);
  }
}
function apply(next: Accent, animate = true) {
  if (showing === next) return;
  showing = next;
  const from = displayed, to = endpoint(next);
  cancelAnimationFrame(frame);
  frame = 0;
  selectSurface(root, next);
  if (!animate || motion.matches || document.hidden || from.every((color, i) => color === to[i])) {
    present(to, true);
  } else {
    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, Math.max(0, (now - start) / 180));
      const ease = 1 - (1 - t) ** 3;
      present([mix(from[0], to[0], ease), mix(from[1], to[1], ease)], t === 1);
      frame = t < 1 ? requestAnimationFrame(step) : 0;
    };
    step(start);
  }
}
function commit(next: Accent, keepDraft = false) {
  committed = next;
  const hex = normalizeHex(next);
  if (hex) custom = hex;
  if (!keepDraft) {
    previewing = false;
    apply(next);
  }
  window.dispatchEvent(new CustomEvent("kui:accent-change", { detail: next }));
}
export function setAccent(next: Accent) {
  if (!isAccent(next)) return;
  commit(next);
  try {
    if (!isAccentPreset(next)) localStorage.setItem(customKey, next);
    localStorage.setItem(storageKey, next);
  } catch { /* The selection still lasts for this page. */ }
}
export function previewCustomAccent(value: string) {
  const hex = normalizeHex(value);
  if (hex) {
    previewing = true;
    apply(hex, false);
  }
}
export function cancelAccentPreview() {
  previewing = false;
  apply(committed);
}
window.addEventListener("storage", event => {
  if (event.key === customKey || event.key === null) custom = normalizeHex(event.newValue) ?? accents.blue[0];
  if (event.key === storageKey || event.key === null) {
    commit(isAccent(event.newValue) ? event.newValue : "neutral", previewing);
  }
});
motion.addEventListener("change", () => {
  if (motion.matches) { cancelAnimationFrame(frame); frame = 0; present(endpoint(showing), true); }
});
document.addEventListener("astro:before-swap", event => {
  // The router replaces root attributes even when it preserves the GPUI window.
  selectSurface(event.newDocument.documentElement, committed);
});
