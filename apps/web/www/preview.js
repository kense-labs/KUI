import init, { start, show_story, install_input_font, set_preferences, set_icon_intent } from "./pkg/kui_web.js";

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
let paintedDark;
let paintedStory;
let paintRevision = 0;
const waiters = new Set();
const status = document.getElementById("status");

// Only a textarea with more content in the direction of travel consumes the wheel.
let editorScroll;
window.kuiEditorScroll = (x,y,w,h,up,down) => { editorScroll = {x,y,w,h,up,down}; };
if (frame?.matches("iframe.rust-story[data-story]")) {
  window.addEventListener("wheel", event => {
    const r = editorScroll;
    if (selected.story === "textarea" && r && !event.ctrlKey && !event.metaKey &&
        event.clientX >= r.x && event.clientX < r.x + r.w && event.clientY >= r.y &&
        event.clientY < r.y + r.h && (event.deltaY > 0 ? r.down : event.deltaY < 0 && r.up)) return;
    event.stopPropagation();
  }, {capture: true, passive: true});
}

// The GPU buffer may be discarded between frames. Keep the most recently
// presented image on the GPU so pressing a theme key never reads pixels back.
let lastPicture;
let pictureVersion = 0;
const copies = new WeakMap();
function rememberPicture() {
  const source = document.querySelector("canvas");
  if (!frame || document.hidden || frame.closest("[hidden], details:not([open])") || !source?.width || !source.height) return;
  if (!lastPicture) lastPicture = document.createElement("canvas");
  if (lastPicture.width !== source.width) lastPicture.width = source.width;
  if (lastPicture.height !== source.height) lastPicture.height = source.height;
  const ctx = lastPicture.getContext("2d");
  if (ctx) { ctx.globalCompositeOperation = "copy"; ctx.drawImage(source, 0, 0); ++pictureVersion; }
}
function copyPicture(destination) {
  if (!lastPicture) return false;
  if (copies.get(destination) === pictureVersion) return true;
  destination.width = lastPicture.width;
  destination.height = lastPicture.height;
  destination.getContext("2d")?.drawImage(lastPicture,0,0);
  copies.set(destination,pictureVersion);
  return true;
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

const textFieldStories = new Set(["input","textarea","password-field","password-strength","search-field","expanding-search","inline-edit"]);
let inputFont;
function warmInputFont(story) {
  if (running && textFieldStories.has(story)) inputFont ??= fetch(new URL("./fonts/KUIInputSans.ttf", import.meta.url))
    .then(async res => {
      if (!res.ok || !install_input_font(new Uint8Array(await res.arrayBuffer()))) throw new Error("Input font unavailable");
    }).catch(error => { inputFont = undefined; console.warn(error.message); });
}
function show(story, lang) {
  warmInputFont(story);
  if (selected.story !== story || selected.lang !== lang) { cancelGesture(); themeTarget = undefined; editorScroll = undefined; lastPicture = undefined; }
  selected = { story, lang };
  document.documentElement.dataset.story = story.split(":")[0];
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  if (!running || (mounted?.story === story && mounted?.lang === lang)) return;
  if (frame && story.split(":")[0] === "theme-switch") set_icon_intent(dark);
  if (show_story(story, lang === "zh")) {
    mounted = selected;
    const address = new URL(location.href);
    address.searchParams.set("story", story);
    address.searchParams.set("lang", lang);
    history.replaceState(null, "", address);
  }
}

function finishWaiters() {
  for (const resolve of waiters) resolve();
  waiters.clear();
}

function preferences(nextDark, nextReduced, waitForPaint = true) {
  if (dark !== nextDark) {
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
  if (height > 0 && frame?.classList.contains("rust-story") && frame.style.height !== height + "px") {
    frame.style.height = height + "px";
  }
  if (frame) queueMicrotask(rememberPicture);
  if (renderedDark === paintedDark && mounted === paintedStory && !waiters.size) return;
  const revision = paintRevision;
  const story = mounted;
  queueMicrotask(() => {
    if (revision !== paintRevision || renderedDark !== dark || story !== mounted) return;
    paintedDark = renderedDark;
    paintedStory = story;
    status?.remove();
    if (frame) {
      frame.dataset.renderedTheme = renderedDark ? "dark" : "light";
      frame.dataset.renderedStory = story?.story ?? "";
    }
    finishWaiters();
  });
};

window.kuiPreviewStarted = () => {
  running = true;
  show(selected.story, selected.lang);
  set_preferences(dark, reduced);
  if (!embedded || !selected.story) status?.remove();
};
window.kuiPreview = { show, preferences, copyPicture, controlRadius: () => themeRadius, iconIntent: dark => { if (running) set_icon_intent(dark); } };
motion.addEventListener("change", () => preferences(dark, motion.matches));
window.addEventListener("pagehide", finishWaiters, { once: true });
setPageColor();
show(selected.story, selected.lang);
status.textContent = selected.lang === "zh" ? "正在加载…" : "Loading…";

try {
  await init();
  mounted = selected;
  start(mounted.story, dark, reduced, mounted.lang === "zh", embedded);
} catch (error) {
  status.textContent = selected.lang === "zh" ? "预览未能加载，请刷新重试。" : "Preview could not load. Refresh to retry.";
  finishWaiters();
  console.error(error);
}
