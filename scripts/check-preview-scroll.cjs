// Against the built KUI site: node scripts/check-preview-scroll.cjs [base]
// Exercises both real ruler scroll regions, navigation cleanup and page scrolling.
const assert = require("node:assert/strict");
const { chromium } = require("playwright");
const base = new URL(process.argv[2] || "http://127.0.0.1:4321");

async function check() {
  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    args: ["--enable-webgpu", "--enable-unsafe-swiftshader", "--use-angle=swiftshader"],
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 880 }, reducedMotion: "reduce" });
  // Observe actual GPUI paint registrations; do not replace the bridge itself.
  await page.addInitScript(() => {
    let callback;
    const seen = [];
    Object.defineProperty(window, "__kuiTestHorizontalRegions", { value: seen });
    Object.defineProperty(window, "kuiHorizontalScroll", {
      configurable: true,
      set(fn) { callback = fn; },
      get() {
        return (...args) => {
          const [x, y, w, h] = args;
          if (w > 0 && h > 0 && !seen.some(r => Math.abs(r.x - x) < 1 && Math.abs(r.y - y) < 1)) {
            seen.push({ x, y, w, h });
          }
          return callback?.(...args);
        };
      },
    });
  });
  try {
    await page.goto(new URL("/kui/components/ruler-slider/", base).href);
    await page.waitForSelector('iframe[data-rendered-story="ruler-slider"]');
    const frame = page.locator('iframe[data-rendered-story="ruler-slider"]');
    await page.waitForFunction(() => {
      const iframe = document.querySelector('iframe[data-rendered-story="ruler-slider"]');
      return iframe?.contentWindow?.__kuiTestHorizontalRegions?.length >= 2;
    });
    const regions = await frame.evaluate(el => el.contentWindow.__kuiTestHorizontalRegions.slice(0, 2));
    for (const [index, region] of regions.entries()) {
      await frame.evaluate((el, y) => window.scrollTo({ top: window.scrollY + el.getBoundingClientRect().top + y - 360, behavior: "instant" }), region.y);
      const box = await frame.boundingBox();
      const x = box.x + region.x + region.w / 2;
      const y = box.y + region.y + region.h / 2;
      const clip = { x: box.x + region.x + 8, y: box.y + region.y + 5, width: Math.min(region.w - 16, 260), height: Math.min(region.h - 10, 50) };
      await page.mouse.move(x, y);
      const before = await page.screenshot({ clip });
      const top = await page.evaluate(() => scrollY);
      await page.mouse.wheel(100, 0);
      const deadline = Date.now() + 3000;
      while ((await page.screenshot({ clip })).equals(before)) {
        assert.ok(Date.now() < deadline, `ruler ${index + 1} did not react to horizontal wheel`);
      }
      assert.equal(await page.evaluate(() => scrollY), top, "horizontal wheel moved the document");
    }
    // The expanded tree example exceeds its viewport and must scroll continuously.
    await page.locator('a[data-nav-item][href$="/tree-table/"]').click();
    await page.waitForSelector('iframe[data-rendered-story="tree-table"]');
    const tree = page.locator('iframe[data-rendered-story="tree-table"]');
    await tree.evaluate(el => el.scrollIntoView({ block: "center", behavior: "instant" }));
    let treeBox = await tree.boundingBox();
    const treeX = treeBox.x + treeBox.width / 2, treeY = treeBox.y + treeBox.height / 2;
    const treeClip = { x: treeX - 110, y: treeY - 45, width: 180, height: 90 };
    await page.mouse.move(treeX, treeY);
    const treeBefore = await page.screenshot({ clip: treeClip });
    const treeTop = await page.evaluate(() => scrollY);
    await page.mouse.wheel(0, 180);
    const treeDeadline = Date.now() + 3000;
    while ((await page.screenshot({ clip: treeClip })).equals(treeBefore)) {
      assert.ok(Date.now() < treeDeadline, "tree table did not repaint after scrolling");
    }
    assert.equal(await page.evaluate(() => scrollY), treeTop, "wheel inside the tree moved the document");
    // Reuse the same preview iframe for a non-scrollable button story.
    await page.locator('a[data-nav-item][href$="/button/"]').click();
    await page.waitForSelector('iframe[data-rendered-story="button"]');
    const plain = page.locator('iframe[data-rendered-story="button"]');
    await plain.evaluate(el => {
      el.contentWindow.__kuiDeliveredWheels = 0;
      el.contentDocument.addEventListener("wheel", () => el.contentWindow.__kuiDeliveredWheels++, { passive: true });
    });
    await plain.evaluate((el, y) => window.scrollTo({ top: window.scrollY + el.getBoundingClientRect().top + y - 360, behavior: "instant" }), regions[0].y);
    let box = await plain.boundingBox();
    await page.mouse.move(box.x + regions[0].x + regions[0].w / 2, box.y + regions[0].y + regions[0].h / 2);
    await page.mouse.wheel(100, 0);
    await page.waitForTimeout(100);
    assert.equal(await plain.evaluate(el => el.contentWindow.__kuiDeliveredWheels), 0, "previous story's horizontal region remained active");
    // A vertical wheel on the otherwise empty preview must still scroll the document.
    box = await plain.boundingBox();
    await page.mouse.move(box.x + 20, box.y + 20);
    const oldTop = await page.evaluate(() => scrollY);
    await page.mouse.wheel(0, 180);
    await page.waitForFunction(top => scrollY > top, oldTop);
    console.log("Both ruler regions, continuous tree scrolling, navigation cleanup and document wheel scrolling passed.");
  } finally {
    await browser.close();
  }
}
check().catch(error => { console.error(error); process.exitCode = 1; });
