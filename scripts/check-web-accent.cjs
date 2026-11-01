// Run against the built docs. Exercise the GPUI canvas and the surrounding document.
const assert = require("node:assert/strict");
const { chromium } = require("playwright");
const { PNG } = require("pngjs");

async function check() {
  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    args: ["--enable-webgpu", "--enable-unsafe-swiftshader", "--use-angle=swiftshader"],
  });
  try {
    const base = process.argv[2] || "http://127.0.0.1:4321";
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, colorScheme: "light" });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(new URL("/kui/components/button/", base).href);
    const frame = page.locator("iframe[data-rendered-story='button']");
    await frame.waitFor();
    const picker = page.locator("header [data-accent-picker]").first();
    const choose = async value => {
      await picker.locator("[data-accent-trigger]").click();
      await picker.locator(`[data-accent-value="${value}"]`).click();
    };
    const foundFill = async (color, minimum) => {
      // The preview follows shared preferences on its next GPUI paint.
      const deadline = Date.now() + 5000;
      let total;
      do {
        const png = PNG.sync.read(await frame.screenshot());
        total = 0;
        for (let i = 0; i < png.data.length; i += 4) {
          if (color.every((value, offset) => Math.abs(value - png.data[i + offset]) <= 1)) total++;
        }
        if (total >= minimum) return;
      } while (Date.now() < deadline);
      assert.fail(`Expected ${minimum} pixels of ${color}, found ${total}`);
    };
    const settled = async color => {
      await page.waitForFunction(color =>
        document.documentElement.dataset.accent === color &&
        !document.documentElement.style.getPropertyValue("--accent-light"), color);
    };
    // A rapid second choice should start at the color already on screen.
    await picker.locator("[data-accent-trigger]").click();
    const samples = await page.evaluate(async () => {
      const root = document.documentElement;
      const choose = value => document.querySelector(`header [data-accent-value="${value}"]`).click();
      const light = () => getComputedStyle(root).getPropertyValue("--accent-light").trim();
      choose("rose");
      // Sample an actual rendered intermediate frame instead of depending on a timeout.
      for (let i = 0; i < 12 && light() === "#000000"; i++) {
        await new Promise(done => requestAnimationFrame(done));
      }
      const mid = light();
      document.querySelector("header [data-accent-trigger]").click();
      choose("blue");
      return [mid, light()];
    });
    assert.notEqual(samples[0], "#000000");
    assert.notEqual(samples[0], "#b93c67");
    assert.equal(samples[0], samples[1], "retarget from the last displayed sample");
    await settled("blue");
    await foundFill([36,91,196], 350);
    await page.reload();
    await frame.waitFor();
    await settled("blue");
    await foundFill([36,91,196], 350);
    await page.locator("header [data-theme-toggle]").first().click();
    await page.waitForFunction(() => document.querySelector("iframe[data-rendered-theme=dark]"));
    await page.waitForTimeout(480);
    await foundFill([139,180,255], 350);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await choose("teal");
    assert.equal(await page.locator("html").evaluate(el => el.style.getPropertyValue("--accent-dark")), "");
    await foundFill([104,212,196], 350);
    await choose("neutral");
    await settled("neutral");
    const editor = picker.locator("[data-custom-accent-editor]");
    const hex = editor.locator("[data-hex-input]");
    const saved = () => page.evaluate(() => localStorage.getItem("kui-accent"));
    await choose("custom");
    await hex.fill("#3366CC");
    assert.equal(await editor.locator("[data-saturation]").inputValue(), "75");
    assert.equal(await editor.locator("[data-brightness]").inputValue(), "80");
    assert.equal(await editor.locator("[data-hue]").inputValue(), "220");
    await foundFill([51,102,204], 350);
    const board = await editor.locator("[data-sv-surface]").boundingBox();
    await page.mouse.move(board.x + board.width / 2, board.y + board.height / 2);
    await page.mouse.down();
    await page.mouse.move(board.x + board.width + 5, board.y - 5, { steps: 4 });
    await page.mouse.up();
    assert.equal(await hex.inputValue(), "#0055FF", "a drag captured past the edge clamps to full saturation and brightness");
    await foundFill([0,85,255], 350);
    await page.keyboard.press("Shift+ArrowDown");
    assert.equal(await editor.locator("[data-brightness]").inputValue(), "90");
    assert.equal(await saved(), "neutral", "dragging or typing must not persist a draft");

    await hex.fill("#1x4");
    assert.equal(await hex.getAttribute("aria-invalid"), "true");
    assert.ok(await editor.locator("[data-custom-apply]").isDisabled());
    await hex.press("Enter");
    assert.ok(await editor.isVisible());
    await hex.press("Escape");
    await settled("neutral");
    assert.equal(await saved(), "neutral");
    assert.ok(await picker.locator("[data-accent-trigger]").evaluate(el => el === document.activeElement));

    await choose("custom");
    await hex.fill("#a3e");
    await editor.locator("[data-custom-apply]").click();
    await settled("#aa33ee");
    await foundFill([170,51,238], 350);
    await page.reload();
    await frame.waitFor();
    await settled("#aa33ee");
    await foundFill([170,51,238], 350);
    assert.equal(await picker.locator('[data-accent-value="custom"]').getAttribute("aria-checked"), "true");

    await choose("teal");
    await choose("custom");
    assert.equal(await hex.inputValue(), "#AA33EE", "reopening remembers the last confirmed custom color");
    for (const [draft, canonical, ink] of [
      ["#fff", "#ffffff", "rgb(0, 0, 0)"],
      ["#000", "#000000", "rgb(255, 255, 255)"],
    ]) {
      await hex.fill(draft);
      await settled(canonical);
      assert.equal(await editor.locator("[data-custom-apply]").evaluate(el => getComputedStyle(el).color), ink);
    }
    await editor.locator("[data-custom-cancel]").click();
    await settled("teal");
    await foundFill([104,212,196], 350);
    assert.equal(await saved(), "teal");
    assert.deepEqual(errors, []);
    console.log("Accent retargeting, canvas fills, persistence, custom preview, confirm, cancel and invalid HEX passed.");
  } finally {
    await browser.close();
  }
}
check().catch(error => { console.error(error); process.exitCode = 1; });
