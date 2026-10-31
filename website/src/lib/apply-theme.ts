import { previewRuntime } from "./preview-runtime";
import { elementOpening, exitMask, freezeSurface, openControl, roundedOpening, type Opening, type RevealMode } from "./theme-surface";
export type { RevealMode } from "./theme-surface";

type Theme = "dark" | "light";
const root = document.documentElement;
const reduced = matchMedia("(prefers-reduced-motion: reduce)");
const opposite = (theme: Theme): Theme => theme === "dark" ? "light" : "dark";
interface Wipe { from: Theme; to: Theme; intent: Theme; layer: HTMLElement; animation: Animation; counter?: Animation; revision: number }
let active: Wipe | undefined;
let pending: { owner: Element; from: Theme; mode: RevealMode; hole: Opening } | undefined;
const intent = (): Theme => active?.intent ?? (root.dataset.theme === "dark" ? "dark" : "light");

let standby: ReturnType<typeof freezeSurface> | undefined;
let invalidation: MutationObserver | undefined;
function discardStandby() {
  invalidation?.disconnect();
  invalidation = undefined;
  standby?.layer.remove();
  standby = undefined;
}
function warm() {
  if (active || standby || reduced.matches || document.hidden) return;
  standby = freezeSurface(intent());
  invalidation = new MutationObserver(records => {
    if (records.some(r => !(r.target instanceof Element && r.target.closest("[data-kui-surface]")))) discardStandby();
  });
  invalidation.observe(document.body, { subtree: true, childList: true, characterData: true, attributes: true });
}
const osTheme = matchMedia("(prefers-color-scheme: dark)");
let followSystem = false;
try { followSystem = localStorage.getItem("theme") === "system"; } catch { /* Optional storage. */ }
function applyDocument(theme: Theme) {
  root.dataset.theme = theme;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", theme === "dark" ? "#000000" : "#ffffff");
  try { localStorage.setItem("theme", followSystem ? "system" : theme); } catch { /* Optional storage. */ }
}
const previews = () => Array.from(document.querySelectorAll<HTMLIFrameElement>("iframe.rust-story[src]"));
function showIntent(theme: Theme) {
  if (active) root.dataset.kuiThemeIntent = theme;
  else delete root.dataset.kuiThemeIntent;
  queueMicrotask(() => {
    if (intent() !== theme) return;
    for (const f of previews()) {
      if (f.dataset.story === "theme-switch") previewRuntime(f)?.iconIntent(theme === "dark");
    }
  });
}
async function showDocument(theme: Theme) {
  applyDocument(theme);
  // Pointer intent can arrive from a WASM callback. Leave that call stack
  // before updating any of the GPUI windows in the same document.
  await Promise.resolve();
  if (root.dataset.theme !== theme) return;
  const visible: Promise<void>[] = [];
  for (const frame of previews()) {
    const box = frame.getBoundingClientRect();
    const onScreen = !document.hidden && !frame.closest("details:not([open]),[hidden]") &&
      box.width > 0 && box.height > 0 && box.bottom > 0 && box.top < innerHeight && box.right > 0 && box.left < innerWidth;
    const paint = previewRuntime(frame)?.preferences(theme === "dark", reduced.matches, onScreen);
    if (onScreen && paint) visible.push(paint);
  }
  if (visible.length) {
    let timer: number;
    await Promise.race([Promise.all(visible), new Promise<void>(resolve => {
      timer = window.setTimeout(resolve, 350); // Disposal or GPU loss.
    })]).finally(() => clearTimeout(timer));
  }
}
function release(wipe: Wipe) {
  if (active !== wipe) return;
  active = undefined;
  wipe.layer.remove();
  wipe.animation.cancel();
  wipe.counter?.cancel();
  delete root.dataset.kuiThemeTransition;
  delete root.dataset.kuiThemeIntent;
}
function watchEnd(wipe: Wipe) {
  const revision = ++wipe.revision;
  void wipe.animation.finished.then(async () => {
    if (active !== wipe || wipe.revision !== revision) return;
    // At zero the old picture fully covers the page. Restore the live
    // document beneath it before dropping the layer, including its canvases.
    if (wipe.intent === wipe.from) await showDocument(wipe.from);
    if (wipe.revision === revision) release(wipe);
  }).catch(() => { /* Superseded or removed on navigation. */ });
}
function moveTo(theme: Theme, mode: RevealMode, hole?: Opening) {
  if (theme === intent()) return;
  if (active) {
    const wipe = active;
    wipe.intent = theme;
    showIntent(theme);
    if (theme === wipe.to && root.dataset.theme !== theme) void showDocument(theme);
    const rate = theme === wipe.to ? 1 : -1;
    wipe.animation.updatePlaybackRate(rate);
    wipe.counter?.updatePlaybackRate(rate);
    // play() at the start in reverse rewinds to the FAR end. Finish a
    // cancelled first-frame press in place instead.
    if (theme === wipe.from && Number(wipe.animation.currentTime) <= 0) {
      ++wipe.revision;
      wipe.animation.pause();
      wipe.counter?.pause();
      const rev = wipe.revision;
      void showDocument(theme).then(() => { if (wipe.revision === rev) release(wipe); });
    } else {
      wipe.animation.play();
      wipe.counter?.play();
      watchEnd(wipe);
    }
    return;
  }
  const from = intent();
  if (reduced.matches || document.hidden) {
    void showDocument(theme);
    showIntent(theme);
    return;
  }
  const pressedAt = performance.now();
  invalidation?.disconnect();
  invalidation = undefined;
  const surface = standby ?? freezeSurface(from);
  standby = undefined;
  const { layer, viewport } = surface;
  const liveFrames = surface.refreshPictures();
  openControl(viewport, hole, liveFrames);
  layer.dataset.kuiFrozen = from;
  layer.style.opacity = "1";
  const [start, end, duration] = exitMask(mode);
  const timing: KeyframeAnimationOptions = { duration, easing: "cubic-bezier(.22,1,.36,1)", fill: "both" };
  let animation: Animation, counter: Animation | undefined;
  if (mode === "reveal" || mode === "rise") {
    // Slide a viewport-sized clip while counter-translating its pixels. Only
    // transforms run on the compositor; the old page itself stays stationary.
    const axis = mode === "reveal" ? "X" : "Y";
    const distance = mode === "reveal" ? 100 : -100;
    const slide = (el: Element, n: number) => el.animate([{ transform: "translate" + axis + "(0%)" }, { transform: "translate" + axis + "(" + n + "%)" }], timing);
    animation = slide(layer, distance);
    counter = slide(viewport, -distance);
    counter.startTime = pressedAt;
  } else {
    animation = layer.animate([{ clipPath: start }, { clipPath: end }], timing);
  }
  animation.startTime = pressedAt;
  active = { from, to: theme, intent: theme, layer, animation, counter, revision: 0 };
  root.dataset.kuiThemeTransition = "playing";
  void showDocument(theme);
  showIntent(theme);
  watchEnd(active);
}

export function chooseTheme(preference: Theme | "system") {
  followSystem = preference === "system";
  try { localStorage.setItem("theme", preference); } catch { /* Optional storage. */ }
  moveTo(preference === "system" ? (osTheme.matches ? "dark" : "light") : preference, "reveal");
}
osTheme.addEventListener("change", () => { if (followSystem) chooseTheme("system"); });
export function toggleTheme(mode: RevealMode = "reveal", hole?: Opening) {
  followSystem = false;
  moveTo(opposite(intent()), mode, hole);
}
export function togglePreviewTheme(frame: HTMLIFrameElement, bounds: DOMRectReadOnly, mode: RevealMode) {
  toggleTheme(mode, screenBounds(frame, bounds));
}
function screenBounds(frame: HTMLIFrameElement, b: DOMRectReadOnly) {
  const f = frame.getBoundingClientRect();
  const bounds = new DOMRect(f.x + b.x, f.y + b.y, b.width, b.height);
  const radius = previewRuntime(frame)?.controlRadius?.() ?? parseFloat(getComputedStyle(frame).getPropertyValue("--radius-control"));
  return roundedOpening(bounds, radius);
}
function begin(owner: Element, mode: RevealMode, hole: Opening) {
  if (pending) end(pending.owner, false);
  pending = { owner, from: intent(), mode, hole };
  toggleTheme(mode, hole);
}
function end(owner: Element, commit: boolean) {
  if (pending?.owner !== owner) return;
  const previous = pending;
  pending = undefined;
  if (!commit) moveTo(previous.from, previous.mode, previous.hole);
}
export function wireThemeKey(key: HTMLButtonElement) {
  key.addEventListener("pointerenter", warm);
  key.addEventListener("focus", warm);
  key.addEventListener("pointerleave", discardStandby);
  key.addEventListener("blur", discardStandby);
  let pointer: number | undefined;
  key.addEventListener("pointerdown", e => {
    if (!e.isPrimary || e.button !== 0) return;
    pointer = e.pointerId;
    key.setPointerCapture(pointer);
    begin(key, "reveal", elementOpening(key));
  });
  key.addEventListener("pointerup", e => {
    if (e.pointerId !== pointer) return;
    pointer = undefined;
    end(key, key.contains(document.elementFromPoint(e.clientX, e.clientY)));
  });
  key.addEventListener("lostpointercapture", () => { if (pointer !== undefined) { pointer = undefined; end(key, false); } });
  // Keyboard and accessibility activation have no preceding pointer press.
  key.addEventListener("click", e => { if (e.detail === 0) toggleTheme("reveal", elementOpening(key)); });
}
window.addEventListener("kui:theme-gesture", event => {
  const { frame, phase, mode, bounds } = (event as CustomEvent).detail;
  if (!(frame instanceof HTMLIFrameElement) || !frame.isConnected || frame.dataset.story !== "theme-switch") return;
  if (phase === "cool") { discardStandby(); return; }
  if (phase === "warm") { warm(); return; }
  if (phase === "begin" && bounds) begin(frame, mode, screenBounds(frame, bounds));
  else end(frame, phase === "commit");
});
function settle() {
  const wipe = active;
  if (!wipe) return;
  void showDocument(wipe.intent);
  release(wipe);
}
// Focusing an embedded canvas also blurs its parent window. Its own pointer
// capture handles cancellation; a parent blur must not undo that first press.
window.addEventListener("blur", () => { if (pending?.owner instanceof HTMLButtonElement) end(pending.owner, false); });
document.addEventListener("visibilitychange", () => { if (document.hidden) { if (pending) end(pending.owner, false); settle(); } });
window.addEventListener("resize", () => { discardStandby(); settle(); });
window.addEventListener("scroll", e => {
  if (e.target instanceof Element && e.target.closest("[data-kui-surface]")) return;
  discardStandby();
  settle();
}, { capture: true, passive: true });
document.addEventListener("astro:before-preparation", () => { pending = undefined; discardStandby(); settle(); });
reduced.addEventListener("change", () => { if (reduced.matches) { discardStandby(); settle(); } });
