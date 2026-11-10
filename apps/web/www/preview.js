// Fetch in parallel with the module imports; pass the same response to the
// WASM initializer so Safari never downloads the large binary twice.
const wasmDownload = fetch(new URL("./pkg/kui_web_bg.wasm", import.meta.url));
wasmDownload.catch(() => {}); // The startup handler below reports the failure.
let start, show_story, install_input_font, request_snapshot, set_preferences, set_icon_intent, set_accent, set_holo_preferences, photo_pinch;

const params = new URLSearchParams(location.search);
const motion = matchMedia("(prefers-reduced-motion: reduce)");
const embedded = window !== parent;
let frame;
try { frame = window.frameElement; } catch { /* External embedding host. */ }

let restoreArticleScroll;
window.kuiArticleViewport = () => {
  if (!frame) return new Float32Array();
  const rect = frame.getBoundingClientRect();
  const doc = frame.ownerDocument;
  const host = doc.defaultView;
  const fullscreen = frame.closest("[data-preview-expanded]");
  const header = fullscreen?.querySelector(".preview-fullscreen-bar") ?? doc.querySelector("header.topbar");
  const screen = frame.closest("[data-preview-screen]")?.getBoundingClientRect();
  const top = Math.max(header?.getBoundingClientRect().bottom ?? 0, screen?.top ?? 0);
  const bottom = Math.min(host.innerHeight, screen?.bottom ?? host.innerHeight);
  const scale = rect.width > 0 && frame.clientWidth > 0 ? rect.width / frame.clientWidth : 1;
  return new Float32Array([
    Math.max(0, (top - rect.top) / scale),
    Math.min(innerHeight, (bottom - rect.top) / scale)
  ]);
};
window.kuiArticleModal = open => {
  if (!open) {
    restoreArticleScroll?.();
    restoreArticleScroll = undefined;
    return;
  }
  if (restoreArticleScroll || !frame?.matches('iframe.rust-story[data-story="card-to-article"]')) return;
  const root = frame.ownerDocument.documentElement;
  const panel = frame.closest("[data-panel=preview]");
  let saved;
  const unlock = () => {
    if (saved) Object.assign(root.style, saved);
    saved = undefined;
  };
  const sync = () => {
    if (panel?.hidden) { unlock(); return; }
    if (saved) return;
    const { overflowY, scrollbarGutter } = root.style;
    saved = { overflowY, scrollbarGutter };
    root.style.scrollbarGutter = "stable";
    root.style.overflowY = "hidden";
  };
  const observer = new MutationObserver(sync);
  if (panel) observer.observe(panel, { attributes: true, attributeFilter: ["hidden"] });
  sync();
  restoreArticleScroll = () => { observer.disconnect(); unlock(); };
};

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
const transparency = matchMedia("(prefers-reduced-transparency: reduce)");
let orientation, orientationOrigin, orientationActive = false, orientationGeneration = 0;
let orientationFrame;
function reportHoloPreferences() {
  if (running && selected.story === "holo-card")
    set_holo_preferences(orientation?.x ?? 0, orientation?.y ?? 0, !!orientation, transparency.matches);
}
function holoOrientation(event) {
  if (document.hidden || self.kuiStoryInViewport === false || self.kuiStoryPageActive === false || reduced) return;
  if (!Number.isFinite(event.beta) || !Number.isFinite(event.gamma)) return;
  orientationOrigin ??= { x: event.gamma, y: event.beta };
  const roll = (screen.orientation?.angle ?? window.orientation ?? 0) * Math.PI / 180;
  const turn = (a, b) => Math.atan2(Math.sin((a-b)*Math.PI/180), Math.cos((a-b)*Math.PI/180)) * 180 / Math.PI;
  const x = turn(event.gamma, orientationOrigin.x), y = turn(event.beta, orientationOrigin.y);
  orientation = { x: (x*Math.cos(roll)+y*Math.sin(roll))/24, y: (y*Math.cos(roll)-x*Math.sin(roll))/24 };
  orientationFrame ??= requestAnimationFrame(() => {
    orientationFrame = undefined;
    reportHoloPreferences();
  });
}
window.kuiRequestHoloOrientation = async () => {
  if (!window.isSecureContext || orientationActive || !window.DeviceOrientationEvent || selected.story !== "holo-card") return;
  const generation = orientationGeneration;
  try {
    const request = DeviceOrientationEvent.requestPermission;
    if (typeof request === "function" && await request.call(DeviceOrientationEvent) !== "granted") return;
    if (generation !== orientationGeneration || selected.story !== "holo-card") return;
    window.addEventListener("deviceorientation", holoOrientation, { passive: true });
    orientationActive = true;
  } catch { /* Finger tilt remains available without a sensor. */ }
};
function stopHoloOrientation() {
  orientationGeneration++;
  window.removeEventListener("deviceorientation", holoOrientation);
  if (orientationFrame !== undefined) cancelAnimationFrame(orientationFrame);
  orientationFrame = orientation = orientationOrigin = undefined;
  orientationActive = false;
  reportHoloPreferences();
}
transparency.addEventListener("change", reportHoloPreferences);

let paintedDark;
let paintedStory;
let paintRevision = 0;
const waiters = new Set();
const status = document.getElementById("status");
const bootRetryKey = "kui:preview-boot-retry:" + new URL(".", import.meta.url).pathname;
function retryBootNetworkFailure(error) {
  if (running || !(error instanceof TypeError) || !/fetch|import/i.test(error.message)) return false;
  try {
    if (sessionStorage.getItem(bootRetryKey) === "1") return false;
    sessionStorage.setItem(bootRetryKey, "1");
  } catch { return false; }
  status.textContent = selected.lang === "zh" ? "正在重新连接预览…" : "Reconnecting preview…";
  location.reload();
  return true;
}

// Only send a wheel event to GPUI when an inner region can consume its dominant axis.
let editorScroll = [], horizontalScroll = [];
let orbitTouchRegion, controlDialRegion, photoRegion, photoPinch, ticketRegion;
window.kuiTicketTouchRegion = (x,y,w,h) => { ticketRegion = {x,y,w,h}; };
window.kuiControlDialRegion = (x,y,w,h) => { controlDialRegion = w && h ? {x,y,w,h} : undefined; };
window.kuiOrbitTouchRegion = (x,y,w,h) => { orbitTouchRegion = w && h ? {x,y,w,h} : undefined; };
window.kuiEditorScroll = (x,y,w,h,up,down) => { if (w && h) editorScroll.push({x,y,w,h,before:up,after:down}); };
window.kuiHorizontalScroll = (x,y,w,h,left,right) => { if (w && h) horizontalScroll.push({x,y,w,h,before:left,after:right}); };
window.kuiPhotoScroll = (x,y,w,h,up,down,viewer) => {
  photoRegion = {x,y,w,h,viewer};
  if (!viewer) window.kuiEditorScroll(x,y,w,h,up,down);
};
const containsScrollPoint = (r, x, y) => r && x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;
const canScroll = (r, delta) => r && (delta < 0 ? r.before : delta > 0 && r.after);
function photoTouch(event) {
  if (selected.story !== "photo-grid" || event.touches.length !== 2 || photoRegion?.viewer) return;
  const [a,b] = event.touches;
  if (!photoPinch && ![a,b].every(t => containsScrollPoint(photoRegion,t.clientX,t.clientY))) return;
  const x = Math.max(photoRegion.x, Math.min(photoRegion.x+photoRegion.w-1,(a.clientX+b.clientX)/2));
  const y = Math.max(photoRegion.y, Math.min(photoRegion.y+photoRegion.h-1,(a.clientY+b.clientY)/2));
  const distance = Math.hypot(a.clientX-b.clientX,a.clientY-b.clientY);
  if (distance < 1) return;
  photo_pinch?.(x,y,photoPinch ? distance/photoPinch.distance-1 : 0,photoPinch ? 1 : 0);
  photoPinch = {x,y,distance};
  if (event.cancelable) event.preventDefault();
}
function finishPhotoTouch(event) {
  if (!photoPinch || event.touches.length >= 2) return;
  const {x,y} = photoPinch;
  photoPinch = undefined;
  photo_pinch?.(x,y,0,event.type === "touchcancel" ? 3 : 2);
}
window.addEventListener("touchstart",photoTouch,{capture:true,passive:false});
window.addEventListener("touchmove",photoTouch,{capture:true,passive:false});
window.addEventListener("touchend",finishPhotoTouch,{capture:true,passive:true});
window.addEventListener("touchcancel",finishPhotoTouch,{capture:true,passive:true});
if (frame?.matches("iframe.rust-story[data-story]")) {
  document.documentElement.dataset.embeddedStory = "";
  window.addEventListener("keydown", event => {
    if (event.key === "Escape" && !event.defaultPrevented && frame.closest("[data-preview-expanded]")) {
      parent.postMessage({type:"kui:preview-exit"}, location.origin);
    }
  });
  window.addEventListener("wheel", event => {
    if (selected.story === "photo-grid" && event.ctrlKey && !photoRegion?.viewer && containsScrollPoint(photoRegion,event.clientX,event.clientY)) return;
    const horizontal = Math.abs(event.deltaX) > Math.abs(event.deltaY);
    const regions = horizontal ? horizontalScroll : editorScroll;
    const delta = horizontal ? event.deltaX : event.deltaY;
    if (!event.ctrlKey && !event.metaKey && regions.some(r => containsScrollPoint(r, event.clientX, event.clientY) && canScroll(r, delta))) return;
    event.stopPropagation();
  }, {capture:true, passive:true});

  // The browser owns gestures on the document and fixed menu chrome. A pan
  // starting on a scrollable row is handled by the GPUI touch recognizer.
  let listPan, orbitPan, dialPan, photoPan, ticketPan;
  const listPanStories = new Set(["team-switcher", "phone-input", "control-center", "mega-menu-morph", "morph-select", "rich-text-editor", "date-reel", "stretch-refresh", "card-to-article", "photo-grid"]);
  window.addEventListener("touchstart", e => {
    const t = e.touches[0];
    ticketPan = selected.story === "ticket-tear" && e.touches.length === 1
      && containsScrollPoint(ticketRegion, t.clientX, t.clientY);
    photoPan = selected.story === "photo-grid" && photoRegion?.viewer && containsScrollPoint(photoRegion,t.clientX,t.clientY);
    const orbit = orbitTouchRegion;
    dialPan = selected.story === "control-center" && e.touches.length === 1
      && containsScrollPoint(controlDialRegion, t.clientX, t.clientY);
    orbitPan = selected.story === "orbit-menu" && e.touches.length === 1 && orbit
      && Math.hypot(t.clientX - orbit.x - orbit.w / 2, t.clientY - orbit.y - orbit.h / 2) <= orbit.w / 2;
    const sideways = selected.story === "orbit-logos";
    const region = (sideways ? horizontalScroll : editorScroll).find(r => containsScrollPoint(r, t.clientX, t.clientY));
    listPan = (sideways || listPanStories.has(selected.story)) && e.touches.length === 1 && region
      ? { x: t.screenX, y: t.screenY, region, sideways, owned: false } : undefined;
  }, {capture: true, passive: true});
  window.addEventListener("touchmove", e => {
    if ((orbitPan && selected.story === "orbit-menu" || dialPan && selected.story === "control-center" || photoPan && selected.story === "photo-grid" || ticketPan && selected.story === "ticket-tear") && e.touches.length === 1) {
      if (e.cancelable) e.preventDefault();
      return;
    }
    if (!listPan || e.touches.length !== 1 || !(listPan.sideways ? selected.story === "orbit-logos" : listPanStories.has(selected.story))) return;
    const t = e.touches[0];
    const dx = listPan.x - t.screenX;
    const dy = listPan.y - t.screenY;
    const delta = listPan.sideways ? dx : dy;
    const cross = listPan.sideways ? dy : dx;
    if (!listPan.owned) {
      if (delta === 0) return;
      if (Math.abs(cross) > Math.abs(delta) || !canScroll(listPan.region, delta)) {
        listPan = undefined;
        return;
      }
      listPan.owned = true;
    }
    if (e.cancelable) e.preventDefault();
    // Keep the iframe stationary for this gesture. A new outward swipe at
    // the edge belongs to the browser, including its native momentum.
  }, {capture: true, passive: false});
  window.addEventListener("touchend", () => { listPan = orbitPan = dialPan = photoPan = ticketPan = undefined; }, {passive: true});
  window.addEventListener("touchcancel", () => { listPan = orbitPan = dialPan = photoPan = ticketPan = undefined; }, {passive: true});
  window.addEventListener("pointerdown", e => {
    if (selected.story === "ticket-tear" && e.isPrimary && e.button === 0
      && e.target instanceof HTMLCanvasElement && containsScrollPoint(ticketRegion, e.clientX, e.clientY)) {
      e.target.setPointerCapture(e.pointerId);
    }
  }, {capture: true, passive: true});

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
function themeMode(story) {
  if (story === "theme-switch") return "reveal";
  return /^theme-switch[-:](eclipse|split|rise)$/.exec(story)?.[1];
}
window.kuiThemeTarget = (x, y, w, h, radius) => {
  themeTarget = new DOMRect(x, y, w, h);
  themeRadius = radius;
};
const hitsKey = e => themeTarget && e.clientX >= themeTarget.left && e.clientX < themeTarget.right &&
  e.clientY >= themeTarget.top && e.clientY < themeTarget.bottom;
function gesture(phase) {
  if (frame) parent.dispatchEvent(new CustomEvent("kui:theme-gesture", {
    detail: {frame, bounds: themeTarget, mode: themeMode(selected.story), phase}
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
    const over = Boolean(themeMode(selected.story) && hitsKey(e));
    if (over && !overKey) gesture("warm");
    if (!over && overKey) gesture("cool");
    overKey = over;
  }, {capture:true,passive:true});
  window.addEventListener("pointerleave", () => { if (overKey) gesture("cool"); overKey = false; });
  window.addEventListener("pointerdown", e => {
    if (!e.isPrimary || e.button !== 0) return;
    consumeActivation = false;
    if (!themeMode(selected.story) || !hitsKey(e)) return;
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

const textFieldStories = new Set(["input", "textarea", "password-field", "password-strength", "search-field", "expanding-search", "inline-edit", "combobox", "morph-select", "scrub-input", "number-field", "tag-input", "multi-select", "mention-input", "emoji-picker", "color-picker", "phone-input", "rich-text-editor", "time-picker", "chat-thread", "ai-command", "agent-plan", "comment-thread", "provider-keys", "team-switcher", "share-access", "json-viewer", "data-grid", "confirm-popover", "stepper", "image-generation", "enrich-column"]);
let inputFont;
function warmInputFont(story) {
  if (running && textFieldStories.has(story)) inputFont ??= fetch(new URL("./fonts/KUIInputSans.ttf", import.meta.url))
    .then(async res => {
      if (!res.ok || !install_input_font(new Uint8Array(await res.arrayBuffer()))) throw new Error("Input font unavailable");
    }).catch(error => { inputFont = undefined; console.warn(error.message); });
}
function show(story, lang) {
  warmInputFont(story);
  if (selected.story === "card-to-article" && (selected.story !== story || selected.lang !== lang)) window.kuiArticleModal(false);
  if (selected.story !== story || selected.lang !== lang) { cancelGesture(); themeTarget = undefined; editorScroll = []; horizontalScroll = []; orbitTouchRegion = controlDialRegion = photoRegion = photoPinch = ticketRegion = undefined; preparePicture(false); }
  if (selected.story === "holo-card" && story !== "holo-card") stopHoloOrientation();
  selected = { story, lang };
  document.documentElement.dataset.story = story.split(":")[0];
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  if (!running) return;
  if (mounted?.story === story && mounted?.lang === lang) return;
  if (frame && themeMode(story)) set_icon_intent(dark);
  if (show_story(story, lang === "zh")) {
    delete document.documentElement.dataset.previewError;
    mounted = selected;
    reportHoloPreferences();
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
  // This hook runs during scene prepaint; controls register their current regions
  // during the following paint, so geometry from previous frames cannot linger.
  editorScroll = [];
  horizontalScroll = [];
  ticketRegion = undefined;
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
  try { sessionStorage.removeItem(bootRetryKey); } catch { /* Storage can be disabled. */ }
  set_accent(...accentPair);
  show(selected.story, selected.lang);
  set_preferences(dark, reduced);
  if (!embedded || !selected.story) status.hidden = true;
};
window.kuiPreview = { show, preferences, accent, preparePicture, copyPicture, controlRadius: () => themeRadius, iconIntent: dark => { if (running) set_icon_intent(dark); } };
motion.addEventListener("change", () => preferences(dark, motion.matches));
window.addEventListener("pagehide", () => { if (orientationActive) stopHoloOrientation(); window.kuiArticleModal(false); editorScroll = []; horizontalScroll = []; preparePicture(false); finishWaiters(); }, { once: true });
setPageColor();
show(selected.story, selected.lang);
status.textContent = selected.lang === "zh" ? "正在加载预览…" : "Loading preview…";

try {
  let init;
  ({ default: init, start, show_story, install_input_font, request_snapshot, set_preferences, set_icon_intent, set_accent, set_holo_preferences, photo_pinch } = await import("./pkg/kui_web.js"));
  await init({ module_or_path: wasmDownload });
  mounted = selected;
  set_accent(...accentPair);
  start(mounted.story, dark, reduced, mounted.lang === "zh", embedded);
} catch (error) {
  if (!retryBootNetworkFailure(error)) {
    document.documentElement.dataset.previewError = "true";
    document.body.removeAttribute("aria-busy");
    status.textContent = selected.lang === "zh" ? "预览未能加载，请刷新重试。" : "Preview could not load. Refresh to retry.";
    finishWaiters();
    console.error(error);
  }
}

// Notify GPUI once per visibility edge; hidden previews must not keep a paint clock alive.
let lastActivity;
function syncPreviewActivity() {
  const active = !document.hidden && self.kuiStoryInViewport !== false && self.kuiStoryPageActive !== false;
  if (active === lastActivity || !running) return;
  lastActivity = active;
  request_snapshot();
}
const visibilityHost = frame?.ownerDocument.defaultView;
let visibilityListeners;
let visibilityCheck;
function refreshFrameVisibility() {
  visibilityCheck = undefined;
  const bounds = frame.getBoundingClientRect();
  self.kuiStoryInViewport = bounds.width > 0 && bounds.height > 0
    && bounds.bottom > 0 && bounds.right > 0
    && bounds.top < visibilityHost.innerHeight && bounds.left < visibilityHost.innerWidth
    && !frame.closest("[hidden], details:not([open])");
  syncPreviewActivity();
}
function scheduleFrameVisibility() {
  visibilityCheck ??= requestAnimationFrame(refreshFrameVisibility);
}
const VisibilityObserver = visibilityHost?.IntersectionObserver ?? IntersectionObserver;
const visibilityObserver = new VisibilityObserver(entries => {
  // Embedded document intersections can go false during focus scrolling,
  // even while the frame is fully on screen. Verify in the host's coordinates.
  if (visibilityHost) {
    scheduleFrameVisibility();
  } else if (entries.length) {
    self.kuiStoryInViewport = entries.at(-1).isIntersecting;
    syncPreviewActivity();
  }
});
function watchPreviewVisibility() {
  visibilityObserver.observe(frame ?? document.documentElement);
  if (visibilityHost && !visibilityListeners) {
    visibilityListeners = new AbortController();
    const options = { capture: true, passive: true, signal: visibilityListeners.signal };
    visibilityHost.addEventListener("scroll", scheduleFrameVisibility, options);
    visibilityHost.addEventListener("resize", scheduleFrameVisibility, options);
    scheduleFrameVisibility();
  }
}
document.addEventListener("visibilitychange", syncPreviewActivity);
window.addEventListener("pagehide", () => {
  self.kuiStoryPageActive = false;
  visibilityObserver.disconnect();
  visibilityListeners?.abort();
  visibilityListeners = undefined;
  if (visibilityCheck !== undefined) cancelAnimationFrame(visibilityCheck);
  visibilityCheck = undefined;
  syncPreviewActivity();
});
window.addEventListener("pageshow", () => {
  self.kuiStoryPageActive = true;
  watchPreviewVisibility();
  syncPreviewActivity();
});
watchPreviewVisibility();
syncPreviewActivity();

// WebKit may resize the GPU surface before GPUI has consumed the platform
// callback. Reconcile logical bounds and scale after the resize observers run.
// This only runs on geometry changes, never as an idle animation loop.
let viewportSyncQueued = false;
function syncPreviewViewport() {
  if (!running || viewportSyncQueued) return;
  viewportSyncQueued = true;
  queueMicrotask(() => {
    viewportSyncQueued = false;
    if (running) request_snapshot();
  });
}
new ResizeObserver(syncPreviewViewport).observe(document.documentElement);
window.addEventListener("resize", syncPreviewViewport);
