import { normalizeHex } from "./accent-palette";
import { cancelAccentPreview, getAccent, getCustomAccent, previewCustomAccent, setAccent } from "./apply-accent";

type Hsv = { h: number; s: number; v: number };
const clamp = (n: number) => Math.max(0, Math.min(1, n));

function fromHex(hex: string, fallbackHue: number): Hsv {
  const [r, g, b] = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255);
  const max = Math.max(r, g, b);
  const delta = max - Math.min(r, g, b);
  let h = fallbackHue;
  if (delta !== 0) {
    const sector = max === r ? (g - b) / delta : max === g ? 2 + (b - r) / delta : 4 + (r - g) / delta;
    h = (60 * sector + 360) % 360;
  }
  return { h, s: max === 0 ? 0 : delta / max, v: max };
}

function toHex({ h, s, v }: Hsv): `#${string}` {
  const c = v * s;
  const sector = (h % 360) / 60;
  const x = c * (1 - Math.abs(sector % 2 - 1));
  const parts = [[c, x, 0], [x, c, 0], [0, c, x], [0, x, c], [x, 0, c], [c, 0, x]][Math.floor(sector)];
  return `#${parts.map(n => Math.round(255 * (n + v - c)).toString(16).padStart(2, "0")).join("")}`;
}

export function wireCustomEditor(panel: HTMLElement, back: () => void, close: () => void) {
  const plane = panel.querySelector<HTMLElement>("[data-sv-surface]")!;
  const sat = panel.querySelector<HTMLInputElement>("[data-saturation]")!;
  const val = panel.querySelector<HTMLInputElement>("[data-brightness]")!;
  const hue = panel.querySelector<HTMLInputElement>("[data-hue]")!;
  const input = panel.querySelector<HTMLInputElement>("[data-hex-input]")!;
  const error = panel.querySelector<HTMLElement>("[data-hex-error]")!;
  const apply = panel.querySelector<HTMLButtonElement>("[data-custom-apply]")!;
  const satText = panel.querySelector<HTMLElement>("[data-saturation-text]")!;
  const valText = panel.querySelector<HTMLElement>("[data-brightness-text]")!;
  const hueText = panel.querySelector<HTMLElement>("[data-hue-text]")!;
  let hsv: Hsv = { h: 0, s: 0, v: 0 };
  let editing = false;
  let drag: number | undefined;
  let previewFrame = 0;
  let lastPreview = "";

  function valid(ok: boolean) {
    input.setAttribute("aria-invalid", String(!ok));
    error.hidden = ok;
    apply.disabled = !ok;
  }

  function refresh(keepInput = false) {
    const hex = toHex(hsv);
    if (!keepInput) input.value = hex.toUpperCase();
    valid(true);
    sat.value = String(Math.round(hsv.s * 100));
    val.value = String(Math.round(hsv.v * 100));
    hue.value = String(Math.round(hsv.h));
    satText.textContent = sat.value + "%";
    valText.textContent = val.value + "%";
    hueText.textContent = hue.value + "°";
    sat.setAttribute("aria-valuetext", satText.textContent);
    val.setAttribute("aria-valuetext", valText.textContent);
    hue.setAttribute("aria-valuetext", hueText.textContent);
    panel.style.setProperty("--hue", String(hsv.h));
    panel.style.setProperty("--saturation", (hsv.s * 100) + "%");
    panel.style.setProperty("--top", ((1 - hsv.v) * 100) + "%");
    panel.style.setProperty("--draft", hex);
    if (!previewFrame) {
      previewFrame = requestAnimationFrame(() => {
        previewFrame = 0;
        const next = toHex(hsv);
        if (editing && next !== lastPreview) {
          lastPreview = next;
          previewCustomAccent(next);
        }
      });
    }
  }

  function pointAt(event: PointerEvent) {
    const b = plane.getBoundingClientRect();
    if (!b.width || !b.height) return;
    hsv.s = clamp((event.clientX - b.left) / b.width);
    hsv.v = clamp(1 - (event.clientY - b.top) / b.height);
    refresh();
  }
  plane.addEventListener("pointerdown", event => {
    if (!event.isPrimary || event.button !== 0 || drag !== undefined) return;
    event.preventDefault();
    drag = event.pointerId;
    plane.setPointerCapture(drag);
    sat.focus({ preventScroll: true });
    pointAt(event);
  });
  plane.addEventListener("pointermove", event => {
    if (drag === event.pointerId) pointAt(event);
  });
  plane.addEventListener("pointerup", event => {
    if (drag !== event.pointerId) return;
    pointAt(event);
    drag = undefined;
  });
  plane.addEventListener("lostpointercapture", () => { drag = undefined; });
  plane.addEventListener("keydown", event => {
    const step = event.shiftKey ? 0.1 : 0.01;
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      hsv.s = clamp(hsv.s + (event.key === "ArrowLeft" ? -step : step));
      sat.focus({ preventScroll: true });
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      hsv.v = clamp(hsv.v + (event.key === "ArrowDown" ? -step : step));
      val.focus({ preventScroll: true });
    } else return;
    event.preventDefault();
    refresh();
  });
  // Native range inputs also handle Home/End, PageUp/PageDown and accessibility actions.
  sat.addEventListener("input", () => { hsv.s = sat.valueAsNumber / 100; refresh(); });
  val.addEventListener("input", () => { hsv.v = val.valueAsNumber / 100; refresh(); });
  hue.addEventListener("input", () => { hsv.h = hue.valueAsNumber; refresh(); });
  input.addEventListener("input", () => {
    const hex = normalizeHex(input.value);
    if (hex) {
      hsv = fromHex(hex, hsv.h);
      // Preserve an unfinished six-digit entry even if its first three digits are valid.
      refresh(true);
    } else {
      valid(false);
    }
  });
  function save() {
    const hex = normalizeHex(input.value);
    if (!editing || !hex) return;
    cancelAnimationFrame(previewFrame);
    previewFrame = 0;
    editing = false;
    if (hex === getAccent()) cancelAccentPreview();
    else setAccent(hex);
    close();
  }
  apply.addEventListener("click", save);
  panel.querySelector("[data-custom-back]")!.addEventListener("click", back);
  panel.querySelector("[data-custom-cancel]")!.addEventListener("click", close);
  panel.addEventListener("keydown", event => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
    } else if (event.key === "Enter" && event.target === input && !event.isComposing) {
      event.preventDefault();
      save();
    }
  });
  return {
    open() {
      hsv = fromHex(getCustomAccent(), 0);
      editing = true;
      lastPreview = "";
      panel.hidden = false;
      refresh();
      sat.focus({ preventScroll: true });
    },
    dismiss() {
      cancelAnimationFrame(previewFrame);
      previewFrame = 0;
      if (drag !== undefined && plane.hasPointerCapture(drag)) plane.releasePointerCapture(drag);
      drag = undefined;
      if (editing) cancelAccentPreview();
      editing = false;
      panel.hidden = true;
    },
  };
}
