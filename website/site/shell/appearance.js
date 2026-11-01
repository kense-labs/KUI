// Restore the surface before the first paint and before the Wasm host starts.
(() => {
  const root = document.documentElement;
  let theme, accent;
  try {
    theme = localStorage.getItem("theme");
    accent = localStorage.getItem("kui-accent");
  } catch {}
  const dark = theme === "dark" || (theme !== "light" && matchMedia("(prefers-color-scheme: dark)").matches);
  root.dataset.theme = dark ? "dark" : "light";
  document.querySelector('meta[name="theme-color"]').content = dark ? "#000000" : "#ffffff";
  if (/^#[0-9a-f]{6}$/i.test(accent)) {
    root.style.setProperty("--custom-accent", accent);
    const [r, g, b] = [1, 3, 5].map(i => {
      const c = parseInt(accent.slice(i, i + 2), 16) / 255;
      return c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4;
    });
    root.style.setProperty("--custom-ink", .2126 * r + .7152 * g + .0722 * b > .179129 ? "#000" : "#fff");
  }
  if (accent) root.dataset.accent = accent;
})();
