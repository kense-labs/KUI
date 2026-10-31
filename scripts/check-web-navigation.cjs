// Against a built docs site; optionally start with an actual earlier website/dist.
// KUI_PREVIOUS_DIST=/path/to/old/dist node scripts/check-web-navigation.cjs [base]
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");
const { chromium } = require("playwright");

const base = new URL(process.argv[2] || "http://127.0.0.1:4321");
const ids = ["dialog", "popover", "tabs", "combobox", "select", "switch", "resizable-panels", "empty-state"];

async function touchDialog(browser) {
  const page = await browser.newPage({ viewport: { width: 320, height: 820 }, isMobile: true,
    hasTouch: true, colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto(new URL("/kui/components/switch/", base).href);
  await page.waitForSelector("iframe[data-rendered-story=switch]");
  await page.locator("[data-sidebar-search]").fill("Dialog");
  await page.locator('a[data-nav-item][href$="/dialog"]').tap();
  const frame = page.locator("iframe[data-rendered-story=dialog]");
  await frame.scrollIntoViewIfNeeded();
  const box = await frame.boundingBox();
  // Sample the backdrop well outside the panel; focus rings can't affect it.
  const pixel = () => page.screenshot({ clip: { x: box.x + 44, y: box.y + 100, width: 4, height: 4 } });
  const closed = await pixel();
  async function expectBackdrop(open) {
    for (let i = 0; i < 20; i++) {
      const shaded = !(await pixel()).equals(closed);
      if (shaded === open) return;
      await page.evaluate(() => new Promise(requestAnimationFrame));
    }
    assert.fail(open ? "Tap did not open the dialog" : "Escape did not close the dialog after touch");
  }
  await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2 - 15);
  await expectBackdrop(true);
  await page.keyboard.press("Escape");
  await expectBackdrop(false);
  await page.close();
}

async function check() {
  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    args: ["--enable-webgpu", "--enable-unsafe-swiftshader", "--use-angle=swiftshader"],
  });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 880 } });
    const errors = [], loads = [];
    page.on("pageerror", e => errors.push(e.message));
    page.on("request", r => { if (new URL(r.url()).pathname.endsWith("/kui_web_bg.wasm")) loads.push(r.url()); });
    let previous = process.env.KUI_PREVIOUS_DIST;
    if (previous) {
      await page.route(base.origin + "/**", async route => {
        if (!previous) return route.continue();
        let file = path.join(previous, decodeURIComponent(new URL(route.request().url()).pathname));
        if ((await fs.stat(file).catch(() => null))?.isDirectory()) file = path.join(file, "index.html");
        if (!(await fs.stat(file).catch(() => null))?.isFile()) return route.fulfill({ status: 404 });
        await route.fulfill({ path: file });
      });
    }
    const nav = id => page.locator(`a[data-nav-item][href$="/${id}"]`);
    async function displayed(id) {
      // Check the mounted GPUI story, not just the new heading or iframe attributes.
      await page.waitForFunction(id => {
        const slot = document.querySelector("[data-preview-slot]"), frame = slot?.querySelector("iframe");
        return location.pathname.endsWith("/" + id + "/") && frame?.dataset.renderedStory === id &&
          (!slot.dataset.previewSrc || new URL(slot.dataset.previewSrc, document.baseURI).pathname === new URL(frame.src).pathname) &&
          new URL(frame.contentWindow.location.href).searchParams.get("story") === id &&
          frame.contentDocument.querySelector("canvas")?.width > 0;
      }, id);
    }
    await page.goto(new URL("/kui/components/button/", base).href);
    await displayed("button");
    const upgraded = !!previous;
    previous = undefined;
    // One existing link reaches new HTML even if yesterday's sidebar lacks the new items.
    await nav("split-button").click();
    await displayed("split-button");
    for (const id of ids) {
      await nav(id).click();
      await displayed(id);
      assert.ok(await nav(id).evaluate(row => {
        const scroll = row.closest("[data-nav-scroll]").getBoundingClientRect(), box = row.getBoundingClientRect();
        return box.top >= scroll.top - 1 && box.bottom <= scroll.bottom + 1;
      }), id + " should remain visible in the sidebar");
    }
    await page.goBack();
    await displayed("resizable-panels");
    await page.goForward();
    await displayed("empty-state");
    // Dispatch another navigation before the first page fetch can finish.
    await page.evaluate(() => {
      for (const id of ["select", "popover", "dialog"]) document.querySelector(`a[data-nav-item][href$="/${id}"]`).click();
    });
    await displayed("dialog");
    const button = page.locator("[data-view=code]");
    await button.click();
    assert.ok(await page.locator("[data-panel=code]").isVisible());
    await page.locator("[data-view=preview]").click();
    assert.ok(await page.locator("[data-preview-slot] iframe").isVisible());
    const search = page.locator("[data-sidebar-search]");
    await search.fill("ResizablePanels");
    await nav("resizable-panels").click();
    await displayed("resizable-panels");
    assert.equal(loads.length, upgraded ? 2 : 1, "only a change of build should restart WASM");
    assert.deepEqual(errors, []);
    await page.close();
    await touchDialog(browser);
    console.log("Sidebar, eight live stories, rapid navigation, history, preview switching and touch/Escape passed.");
  } finally {
    await browser.close();
  }
}
check().catch(e => { console.error(e); process.exitCode = 1; });
