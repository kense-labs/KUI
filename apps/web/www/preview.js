let start, show_story, install_input_font, request_snapshot, set_preferences, set_icon_intent, set_accent;

const params = new URLSearchParams(location.search);
const motion = matchMedia("(prefers-reduced-motion: reduce)");
const embedded = window !== parent;
let frame;
try { frame = window.frameElement; } catch { /* External embedding host. */ }

let selected = initialSelection();
let mounted;
let running = false;
let dark = initialDark();
let reduced = motion.matches;
let accentPair = initialAccent();
function initialAccent() {
  const host = frame?.ownerDocument.documentElement;
  if (!host?.dataset.accent || host.dataset.accent === "neutral") return [0, 0];
  const styles = host.ownerDocument.defaultView.getComputedStyle(host);
  const sample = prop => {
    const hex = styles.getPropertyValue(prop).trim();
    return /^#[0-9a-f]{6}$/i.test(hex) ? parseInt(hex.slice(1), 16) * 256 + 255 : 0;
  };
  return [sample("--accent-light"), sample("--accent-dark")];
}
function accent(light, dark) {
  accentPair = [light, dark];
  if (running) set_accent(light, dark);
}
let paintedDark;
let paintedStory;
let paintRevision = 0;
const waiters = new Set();
const status = document.getElementById("status");

// Only send a wheel event to GPUI when an inner region can consume its dominant axis.
let editorScroll, horizontalScroll;
window.kuiEditorScroll = (x,y,w,h,up,down) => { editorScroll = w && h ? {x,y,w,h,before:up,after:down} : undefined; };
window.kuiHorizontalScroll = (x,y,w,h,left,right) => { horizontalScroll = {x,y,w,h,before:left,after:right}; };
const containsScrollPoint = (r, x, y) => r && x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;
const canScroll = (r, delta) => r && (delta < 0 ? r.before : delta > 0 && r.after);
if (frame?.matches("iframe.rust-story[data-story]")) {
  document.documentElement.dataset.embeddedStory = "";
  window.addEventListener("wheel", event => {
    const horizontal = Math.abs(event.deltaX) > Math.abs(event.deltaY);
    const r = horizontal ? horizontalScroll : editorScroll;
    const delta = horizontal ? event.deltaX : event.deltaY;
    if (!event.ctrlKey && !event.metaKey && containsScrollPoint(r, event.clientX, event.clientY) && canScroll(r, delta)) return;
    event.stopPropagation();
  }, {capture:true, passive:true});

  // The browser owns gestures on the document and fixed menu chrome. A pan
  // starting on a scrollable row is handled by the GPUI touch recognizer.
  let listPan;
  window.addEventListener("touchstart", e => {
    const t = e.touches[0];
    listPan = selected.story === "team-switcher" && e.touches.length === 1
      && containsScrollPoint(editorScroll, t.clientX, t.clientY)
      ? { x: t.screenX, y: t.screenY, region: editorScroll, owned: false } : undefined;
  }, {capture: true, passive: true});
  window.addEventListener("touchmove", e => {
    if (!listPan || e.touches.length !== 1 || selected.story !== "team-switcher") return;
    const t = e.touches[0];
    const dy = listPan.y - t.screenY;
    if (!listPan.owned) {
      if (dy === 0) return;
      if (Math.abs(t.screenX - listPan.x) > Math.abs(dy) || !canScroll(listPan.region, dy)) {
        listPan = undefined;
        return;
      }
      listPan.owned = true;
    }
    if (e.cancelable) e.preventDefault();
    // Keep the iframe stationary for this gesture. A new outward swipe at
    // the edge belongs to the browser, including its native momentum.
  }, {capture: true, passive: false});
  window.addEventListener("touchend", () => { listPan = undefined; }, {passive: true});
  window.addEventListener("touchcancel", () => { listPan = undefined; }, {passive: true});

  // After a sidebar navigation the host has DOM focus. GPUI handles a touch
  // tap on a button without focusing its DOM keyboard receiver (it only opens
  // the software keyboard for text fields). Reconnect hardware keys on taps.
  let tap;
  window.addEventListener("pointerdown", e => {
    tap = e.isPrimary && e.pointerType === "touch" && e.target instanceof HTMLCanvasElement
      ? { id: e.pointerId, x: e.clientX, y: e.clientY } : undefined;
  }, { passive: true });
  window.addEventListener("pointermove", e => {
    if (tap?.id === e.pointerId && Math.hypot(e.clientX - tap.x, e.clientY - tap.y) > 8) tap = undefined;
  }, { passive: true });
  window.addEventListener("pointercancel", () => { tap = undefined; }, { passive: true });
  window.addEventListener("pointerup", e => {
    const completed = tap?.id === e.pointerId && Math.hypot(e.clientX - tap.x, e.clientY - tap.y) <= 8;
    tap = undefined;
    if (!completed) return;
    const receiver = document.querySelector('body > textarea[inputmode="none"]');
    if (receiver && document.activeElement !== receiver) receiver.focus({ preventScroll: true });
  }, { passive: true });
}

// Request a single bitmap when a theme button receives hover or focus. An
// ordinary gallery frame never reads its GPU canvas into a second surface.
let lastPicture;
let pictureRevision = 0;
let pictureWanted = false;
let picturePending = false;
function discardPicture() {
  ++pictureRevision;
  picturePending = false;
  lastPicture?.close();
  lastPicture = undefined;
}
function preparePicture(wanted) {
  pictureWanted = wanted;
  if (!wanted) { discardPicture(); return; }
  if (!running || picturePending || lastPicture || typeof createImageBitmap !== "function") return;
  picturePending = true;
  // A gesture can originate in a GPUI callback. Return to JS before requesting another draw.
  queueMicrotask(() => { if (picturePending) request_snapshot(); });
}
function capturePicture() {
  const source = document.querySelector("canvas");
  if (!source?.width || !source.height) return;
  picturePending = false;
  const revision = ++pictureRevision;
  createImageBitmap(source).then(bitmap => {
    if (pictureRevision !== revision || !pictureWanted) { bitmap.close(); return; }
    lastPicture = bitmap;
  }).catch(() => { /* The host can keep a booting or lost canvas live. */ });
}
function copyPicture(destination) {
  if (!lastPicture) return false;
  const bitmap = lastPicture;
  lastPicture = undefined;
  destination.width = bitmap.width;
  destination.height = bitmap.height;
  const renderer = destination.getContext("bitmaprenderer");
  if (renderer) { renderer.transferFromImageBitmap(bitmap); return true; }
  const ctx = destination.getContext("2d");
  if (ctx) ctx.drawImage(bitmap, 0, 0);
  bitmap.close();
  return !!ctx;
}

let themeTarget;
let themeRadius;
let themePointer;
let consumeActivation = false;
window.kuiThemeTarget = (x, y, w, h, radius) => {
  themeTarget = new DOMRect(x, y, w, h);
  themeRadius = radius;
};
const hitsKey = e => themeTarget && e.clientX >= themeTarget.left && e.clientX < themeTarget.right &&
  e.clientY >= themeTarget.top && e.clientY < themeTarget.bottom;
function gesture(phase) {
  if (frame) parent.dispatchEvent(new CustomEvent("kui:theme-gesture", {
    detail: {frame, bounds: themeTarget, mode: selected.story.split(":")[1] || "reveal", phase}
  }));
}
function cancelGesture() {
  if (themePointer !== undefined) gesture("cancel");
  themePointer = undefined;
  consumeActivation = false;
}
if (frame) {
  let overKey = false;
  window.addEventListener("pointermove", e => {
    const over = selected.story.split(":")[0] === "theme-switch" && hitsKey(e);
    if (over && !overKey) gesture("warm");
    if (!over && overKey) gesture("cool");
    overKey = over;
  }, {capture:true,passive:true});
  window.addEventListener("pointerleave", () => { if (overKey) gesture("cool"); overKey = false; });
  window.addEventListener("pointerdown", e => {
    if (!e.isPrimary || e.button !== 0) return;
    consumeActivation = false;
    if (selected.story.split(":")[0] !== "theme-switch" || !hitsKey(e)) return;
    themePointer = e.pointerId;
    gesture("begin");
  }, true);
  window.addEventListener("pointerup", e => {
    if (e.pointerId !== themePointer) return;
    themePointer = undefined;
    consumeActivation = true;
    setTimeout(() => { consumeActivation = false; }, 0);
    gesture(hitsKey(e) ? "commit" : "cancel");
  }, true);
  window.addEventListener("pointercancel", cancelGesture, true);
  window.addEventListener("blur", cancelGesture);
  window.addEventListener("keydown", () => { consumeActivation = false; }, true);
}
window.kuiPointerActivated = () => {
  const consumed = consumeActivation;
  consumeActivation = false;
  return consumed;
};

function initialSelection() {
  return {
    story: frame?.dataset.storyKey ?? params.get("story") ?? "",
    lang: frame?.dataset.lang ?? params.get("lang") ?? "en",
  };
}

function initialDark() {
  const hostTheme = frame?.ownerDocument.documentElement.dataset.theme;
  return hostTheme ? hostTheme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches;
}

function setPageColor() {
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
}

const textFieldStories = new Set(["input", "textarea", "password-field", "password-strength", "search-field", "expanding-search", "inline-edit", "combobox", "number-field", "tag-input", "multi-select", "mention-input", "emoji-picker", "color-picker", "phone-input", "rich-text-editor", "time-picker", "chat-thread", "ai-command", "agent-plan", "comment-thread", "provider-keys", "team-switcher", "share-access", "json-viewer", "data-grid", "confirm-popover", "stepper"]);
let inputFont;
function warmInputFont(story) {
  if (running && textFieldStories.has(story)) inputFont ??= fetch(new URL("./fonts/KUIInputSans.ttf", import.meta.url))
    .then(async res => {
      if (!res.ok || !install_input_font(new Uint8Array(await res.arrayBuffer()))) throw new Error("Input font unavailable");
    }).catch(error => { inputFont = undefined; console.warn(error.message); });
}
function show(story, lang) {
  warmInputFont(story);
  if (selected.story !== story || selected.lang !== lang) { cancelGesture(); themeTarget = undefined; editorScroll = horizontalScroll = undefined; preparePicture(false); }
  selected = { story, lang };
  document.documentElement.dataset.story = story.split(":")[0];
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  if (!running) return;
  if (mounted?.story === story && mounted?.lang === lang) return;
  if (frame && story.split(":")[0] === "theme-switch") set_icon_intent(dark);
  if (show_story(story, lang === "zh")) {
    delete document.documentElement.dataset.previewError;
    mounted = selected;
    const address = new URL(location.href);
    address.searchParams.set("story", story);
    address.searchParams.set("lang", lang);
    history.replaceState(null, "", address);
  } else {
    // Do not leave the last interactive control under an unrelated page title.
    mounted = undefined;
    paintedStory = undefined;
    document.documentElement.dataset.previewError = "true";
    finishWaiters();
    if (frame) delete frame.dataset.renderedStory;
    status.hidden = false;
    status.textContent = lang === "zh" ? "此预览未能打开，请刷新页面重试。" : "This preview could not open. Reload the page to retry.";
  }
}

function finishWaiters() {
  for (const resolve of waiters) resolve();
  waiters.clear();
}

function preferences(nextDark, nextReduced, waitForPaint = true) {
  if (dark !== nextDark) {
    discardPicture();
    ++paintRevision;
    paintedDark = undefined;
    finishWaiters();
  }
  dark = nextDark;
  reduced = nextReduced;
  setPageColor();
  if (running) set_preferences(dark, reduced);
  if (!waitForPaint || !running || paintedDark === dark || !embedded) return Promise.resolve();
  return new Promise(resolve => waiters.add(resolve));
}

// GPUI calls this during prepaint. The microtask runs after the draw and
// presentation callback returns, before the browser composites the next frame.
window.kuiFramePainted = (height, renderedDark) => {
  if (document.documentElement.dataset.previewError) return;
  const nextHeight = frame && height > 0 && frame.classList.contains("rust-story") ? height + "px" : "";
  const resize = nextHeight && frame.style.height !== nextHeight;
  if (!picturePending && !resize && renderedDark === paintedDark && mounted === paintedStory && !waiters.size) return;
  const revision = paintRevision;
  const story = mounted;
  queueMicrotask(() => {
    if (revision !== paintRevision || renderedDark !== dark || story !== mounted || document.documentElement.dataset.previewError) return;
    if (picturePending) capturePicture();
    // Resize after presentation. Changing the iframe viewport during prepaint
    // can invalidate WebKit's active GPU surface.
    if (resize && frame.style.height !== nextHeight) frame.style.height = nextHeight;
    paintedDark = renderedDark;
    paintedStory = story;
    status.hidden = true;
    document.body.removeAttribute("aria-busy");
    if (frame) {
      frame.dataset.renderedTheme = renderedDark ? "dark" : "light";
      frame.dataset.renderedStory = story?.story ?? "";
    }
    finishWaiters();
  });
};

window.kuiPreviewStarted = () => {
  running = true;
  set_accent(...accentPair);
  show(selected.story, selected.lang);
  set_preferences(dark, reduced);
  if (!embedded || !selected.story) status.hidden = true;
};
window.kuiPreview = { show, preferences, accent, preparePicture, copyPicture, controlRadius: () => themeRadius, iconIntent: dark => { if (running) set_icon_intent(dark); } };
motion.addEventListener("change", () => preferences(dark, motion.matches));
window.addEventListener("pagehide", () => { preparePicture(false); finishWaiters(); }, { once: true });
setPageColor();
show(selected.story, selected.lang);
status.textContent = selected.lang === "zh" ? "正在加载预览…" : "Loading preview…";

try {
  let init;
  ({ default: init, start, show_story, install_input_font, request_snapshot, set_preferences, set_icon_intent, set_accent } = await import("./pkg/kui_web.js"));
  await init();
  mounted = selected;
  set_accent(...accentPair);
  start(mounted.story, dark, reduced, mounted.lang === "zh", embedded);
} catch (error) {
  document.documentElement.dataset.previewError = "true";
  document.body.removeAttribute("aria-busy");
  status.textContent = selected.lang === "zh" ? "预览未能加载，请刷新重试。" : "Preview could not load. Refresh to retry.";
  finishWaiters();
  console.error(error);
}

// The story clock follows the viewport even when the canvas is inside a scrolling docs page.
new IntersectionObserver(([entry]) => { self.kuiStoryInViewport = entry.isIntersecting; }).observe(document.documentElement);
