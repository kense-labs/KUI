import { previewRuntime } from "./preview-runtime";

type Corners = readonly [number, number, number, number];
export interface Opening { rect: DOMRectReadOnly; corners: Corners }

export function roundedOpening(rect: DOMRectReadOnly, radius: number): Opening {
  return { rect, corners: [radius, radius, radius, radius] };
}

export function elementOpening(element: HTMLElement): Opening {
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  const scale = element.offsetWidth ? rect.width / element.offsetWidth : 1;
  const corner = (value: string) => parseFloat(value) * scale || 0;
  return { rect, corners: [
    corner(style.borderTopLeftRadius), corner(style.borderTopRightRadius),
    corner(style.borderBottomRightRadius), corner(style.borderBottomLeftRadius),
  ] };
}

function openingPath({ rect, corners }: Opening): string {
  const { left, top, right, bottom, width, height } = rect;
  const limit = Math.min(width, height) / 2;
  const [tl, tr, br, bl] = corners.map(r => Math.max(0, Math.min(limit, r)));
  return [
    `M${left + tl} ${top} H${right - tr} A${tr} ${tr} 0 0 1 ${right} ${top + tr}`,
    `V${bottom - br} A${br} ${br} 0 0 1 ${right - br} ${bottom}`,
    `H${left + bl} A${bl} ${bl} 0 0 1 ${left} ${bottom - bl}`,
    `V${top + tl} A${tl} ${tl} 0 0 1 ${left + tl} ${top} Z`,
  ].join(" ");
}

// Freeze only the outgoing presentation. The original document, canvas, focus
// and event targets stay mounted and interactive throughout the wipe.
export function freezeSurface(theme: string) {
  const layer = document.createElement("div");
  layer.dataset.kuiSurface = theme;
  layer.style.opacity = "0";
  layer.inert = true;
  layer.setAttribute("aria-hidden", "true");
  const viewport = document.createElement("div");
  viewport.className = "kui-frozen-viewport";
  layer.append(viewport);
  const scrollPairs = new Map<Element, Element>();
  const pictures = new Map<HTMLIFrameElement, HTMLCanvasElement>();
  const scrollX = window.scrollX, scrollY = window.scrollY;
  const height = innerHeight;
  const copy = (source: Node): Node | undefined => {
    if (source.nodeType === Node.TEXT_NODE) return source.cloneNode(false);
    if (!(source instanceof Element) || source.matches("script,style,link,template,[data-kui-surface],astro-route-announcer")) return;
    if (source instanceof HTMLIFrameElement) {
      const rect = source.getBoundingClientRect();
      const visible = rect.width && rect.height && rect.bottom > 0 && rect.top < height && !source.closest("[hidden],details:not([open])");
      const replacement = document.createElement("canvas");
      if (visible) pictures.set(source, replacement);
      for (const attr of Array.from(source.attributes)) {
        if (attr.name === "class" || attr.name === "style" || attr.name.startsWith("data-astro-cid-")) replacement.setAttribute(attr.name, attr.value);
      }
      return replacement;
    }
    const clone = source.cloneNode(false) as Element;
    clone.removeAttribute("id");
    clone.removeAttribute("autofocus");
    if (source instanceof HTMLInputElement && clone instanceof HTMLInputElement) clone.value = source.value;
    if (source.scrollTop || source.scrollLeft) scrollPairs.set(source, clone);
    if (source.hasAttribute("hidden")) return clone;
    const closed = source instanceof HTMLDetailsElement && !source.open;
    for (const child of Array.from(source.childNodes)) {
      if (closed && !(child instanceof HTMLElement && child.tagName === "SUMMARY")) continue;
      const node = copy(child);
      if (node) clone.append(node);
    }
    return clone;
  };
  for (const child of Array.from(document.body.children)) {
    const clone = copy(child);
    if (clone) viewport.append(clone);
  }
  document.body.append(layer);
  viewport.scrollTo(scrollX, scrollY);
  for (const [source, clone] of scrollPairs) clone.scrollTo(source.scrollLeft, source.scrollTop);
  const refreshPictures = () => {
    const live: Opening[] = [];
    for (const [frame, canvas] of pictures) {
      if (!previewRuntime(frame)?.copyPicture(canvas)) live.push(elementOpening(frame));
    }
    return live;
  };
  refreshPictures();
  return { layer, viewport, refreshPictures };
}

export function openControl(viewport: HTMLElement, control: Opening | undefined, liveFrames: Opening[]) {
  // Cut precisely the outlines painted by GPUI or CSS. A booting preview
  // stays visible until it has supplied a picture for the frozen surface.
  const openings = control ? [...liveFrames, control] : liveFrames;
  if (!openings.length) return;
  const cuts = openings.map(opening => `<path d="${openingPath(opening)}" fill="black"/>`).join("");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${document.documentElement.clientWidth} ${innerHeight}"><mask id="holes"><rect width="100%" height="100%" fill="white"/>${cuts}</mask><rect width="100%" height="100%" fill="white" mask="url(#holes)"/></svg>`;
  viewport.style.maskImage = `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}

export type RevealMode = "reveal" | "eclipse" | "split" | "rise";

// Clip the OLD picture. Inverse masks uncover the already live target page.
export function exitMask(mode: RevealMode): [string, string, number] {
  if (mode === "reveal") return ["inset(0 0 0 0)", "inset(0 0 0 100%)", 380];
  if (mode === "rise") return ["inset(0 0 0 0)", "inset(0 0 100% 0)", 360];
  const cutout = (points: string) => `polygon(evenodd, 0% 0%,100% 0%,100% 100%,0% 100%,0% 0%,${points})`;
  if (mode === "split") {
    const opening = (a: number, b: number) => cutout(`0% ${a}%,0% ${b}%,100% ${b}%,100% ${a}%,0% ${a}%`);
    return [opening(50, 50), opening(0, 100), 390];
  }
  const ellipse = (rx: number) => cutout(Array.from({ length: 97 }, (_, i) => {
    const a = i * Math.PI * 2 / 96;
    return `${(50 + rx * Math.cos(a)).toFixed(3)}% ${(50 + 110 * Math.sin(a)).toFixed(3)}%`;
  }).join(","));
  return [ellipse(0), ellipse(80), 430];
}
