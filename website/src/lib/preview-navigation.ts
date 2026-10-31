import { swapFunctions } from "astro:transitions/client";
import { previewRuntime } from "./preview-runtime";

interface Ancestor {
  current: HTMLElement;
  incoming: HTMLElement;
}

// An iframe reloads when even one of its ancestors is detached. Keep the
// path from body to the preview slot connected and replace its siblings.
function previewPath(slot: HTMLElement, nextSlot: HTMLElement): Ancestor[] | undefined {
  const path: Ancestor[] = [];
  let current: HTMLElement | null = slot;
  let incoming: HTMLElement | null = nextSlot;
  while (current && incoming && current.tagName === incoming.tagName) {
    path.push({current, incoming});
    if (current === document.body) return incoming.tagName === "BODY" ? path : undefined;
    current = current.parentElement;
    incoming = incoming.parentElement;
  }
}

function copyAttributes(current: HTMLElement, incoming: HTMLElement) {
  for (const attr of current.getAttributeNames()) {
    if (!incoming.hasAttribute(attr)) current.removeAttribute(attr);
  }
  for (const {name, value} of incoming.attributes) {
    if (current.getAttribute(name) !== value) current.setAttribute(name, value);
  }
}

function replaceSurroundings({current, incoming}: Ancestor) {
  while (current.previousSibling) current.previousSibling.remove();
  while (current.nextSibling) current.nextSibling.remove();
  while (incoming.previousSibling) current.before(incoming.parentNode!.firstChild!);
  while (incoming.nextSibling) current.after(incoming.parentNode!.lastChild!);
}

function mountStory(slot: HTMLElement, nextSlot: HTMLElement) {
  const next = nextSlot.querySelector<HTMLIFrameElement>("iframe");
  const frame = slot.querySelector<HTMLIFrameElement>("iframe");
  if (!next) return;
  if (!frame) {
    slot.append(next);
    return;
  }
  frame.removeAttribute("title");
  frame.setAttribute("aria-label", next.getAttribute("aria-label") ?? "");
  frame.className = next.className;
  frame.dataset.story = next.dataset.story;
  frame.dataset.storyKey = next.dataset.storyKey;
  frame.dataset.lang = next.dataset.lang;
  // The live document reads these attributes itself if WASM is still starting.
  previewRuntime(frame)?.show(next.dataset.storyKey!, next.dataset.lang!);
}

document.addEventListener("astro:before-swap", (event) => {
  const nextDoc = event.newDocument;
  const theme = document.documentElement.dataset.theme;
  if (theme) nextDoc.documentElement.dataset.theme = theme;
  const slot = document.querySelector<HTMLElement>("[data-preview-slot]");
  const nextSlot = nextDoc.querySelector<HTMLElement>("[data-preview-slot]");
  if (!slot || !nextSlot) return;
  const path = previewPath(slot, nextSlot);
  if (!path) return;

  void event.viewTransition.ready.catch(() => { /* The live preview replaces the route fade. */ });
  event.viewTransition.skipTransition();
  event.swap = () => {
    const restoreFocus = swapFunctions.saveFocus();
    swapFunctions.deselectScripts(nextDoc);
    swapFunctions.swapRootAttributes(nextDoc);
    swapFunctions.swapHeadElements(nextDoc);
    for (const pair of path) {
      copyAttributes(pair.current, pair.incoming);
      if (pair.current !== document.body) replaceSurroundings(pair);
    }
    mountStory(slot, nextSlot);
    restoreFocus();
  };
});
