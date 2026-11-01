import { isAccent, normalizeHex } from "./accent-palette";
import { getAccent, getCustomAccent, setAccent } from "./apply-accent";
import { wireCustomEditor } from "./custom-accent-editor";

const picker = "[data-accent-picker]";
const valueSelector = "[data-accent-value]";
let opened: { root: HTMLElement; dismiss: (restoreFocus: boolean) => void } | undefined;

export function closeAccentPicker(restoreFocus = false) {
  const current = opened;
  opened = undefined;
  current?.dismiss(restoreFocus);
}

function sync(root: HTMLElement) {
  const trigger = root.querySelector<HTMLButtonElement>("[data-accent-trigger]")!;
  const title = root.querySelector<HTMLElement>("[data-accent-menu]")!.getAttribute("aria-label");
  const accent = getAccent();
  const customHex = normalizeHex(accent);
  const swatch = root.querySelector<HTMLElement>("[data-custom-swatch]")!;
  swatch.style.setProperty("--swatch", customHex ?? getCustomAccent());
  for (const item of root.querySelectorAll<HTMLButtonElement>(valueSelector)) {
    const custom = item.dataset.accentValue === "custom";
    const selected = custom ? !!customHex : item.dataset.accentValue === accent;
    item.setAttribute("aria-checked", String(selected));
    if (selected) {
      const label = `${title} · ${item.textContent?.trim()}${customHex ? " · " + customHex.toUpperCase() : ""}`;
      trigger.setAttribute("aria-label", label);
      trigger.title = label;
    }
  }
}

export function wireAccentPickers(closeOtherMenus: () => void) {
  for (const root of document.querySelectorAll<HTMLElement>(picker)) {
    if (root.closest("[data-kui-surface]")) continue;
    sync(root);
    if (root.dataset.wired) continue;
    root.dataset.wired = "1";
    const trigger = root.querySelector<HTMLButtonElement>("[data-accent-trigger]")!;
    const menu = root.querySelector<HTMLElement>("[data-accent-menu]")!;
    const items = Array.from(menu.querySelectorAll<HTMLButtonElement>(valueSelector));

    const custom = menu.querySelector<HTMLButtonElement>('[data-accent-value="custom"]')!;
    const panel = root.querySelector<HTMLElement>("[data-custom-accent-editor]")!;
    const editor = wireCustomEditor(panel, () => {
      root.removeAttribute("data-editing");
      menu.inert = false;
      trigger.setAttribute("aria-controls", menu.id);
      custom.focus({ preventScroll: true });
      editor.dismiss();
    }, () => closeAccentPicker(true));

    function dismiss(restoreFocus: boolean) {
      if (restoreFocus) trigger.focus({ preventScroll: true });
      trigger.setAttribute("aria-expanded", "false");
      trigger.setAttribute("aria-controls", menu.id);
      menu.inert = true;
      editor.dismiss();
      root.removeAttribute("data-editing");
    }

    function open() {
      closeOtherMenus();
      if (opened?.root !== root) closeAccentPicker();
      opened = { root, dismiss };
      menu.inert = false;
      trigger.setAttribute("aria-expanded", "true");
      (items.find(item => item.getAttribute("aria-checked") === "true") ?? items[0]).focus();
    }
    trigger.addEventListener("click", () => {
      if (opened?.root === root) closeAccentPicker(true);
      else open();
    });
    trigger.addEventListener("keydown", event => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      if (root.hasAttribute("data-editing")) panel.querySelector<HTMLInputElement>("[data-saturation]")?.focus();
      else open();
    });
    menu.addEventListener("keydown", event => {
      const index = items.indexOf(document.activeElement as HTMLButtonElement);
      let next = index;
      switch (event.key) {
        case "ArrowUp": next = (index + items.length - 1) % items.length; break;
        case "ArrowDown": next = (index + 1) % items.length; break;
        case "Home": next = 0; break;
        case "End": next = items.length - 1; break;
        case "Escape":
          event.preventDefault();
          event.stopPropagation();
          closeAccentPicker(true);
          return;
        case "Tab":
          // Resume the browser's normal tab sequence from the toolbar trigger.
          closeAccentPicker(true);
          return;
        default: return;
      }
      event.preventDefault();
      items[next].focus();
    });
    for (const item of items) {
      item.addEventListener("click", () => {
        const choice = item.dataset.accentValue;
        if (choice === "custom") {
          root.setAttribute("data-editing", "");
          trigger.setAttribute("aria-controls", panel.id);
          editor.open();
          menu.inert = true;
          return;
        }
        if (isAccent(choice) && choice !== getAccent()) setAccent(choice);
        closeAccentPicker(true);
      });
    }
    root.addEventListener("focusout", event => {
      if (opened?.root === root && event.relatedTarget instanceof Node && !root.contains(event.relatedTarget)) {
        closeAccentPicker();
      }
    });
  }
}

window.addEventListener("kui:accent-change", () => {
  document.querySelectorAll<HTMLElement>(picker).forEach(root => {
    if (!root.closest("[data-kui-surface]")) sync(root);
  });
});
document.addEventListener("pointerdown", event => {
  if (opened && !event.composedPath().includes(opened.root)) closeAccentPicker();
}, true);
window.addEventListener("blur", () => closeAccentPicker());
document.addEventListener("astro:before-swap", () => closeAccentPicker());
