// Run against a built KUI docs site with Playwright and its Chromium installed.
// node scripts/check-web-pinyin.cjs [http://127.0.0.1:4321]
const assert = require("node:assert/strict");
const { chromium } = require("playwright");

async function check() {
  const browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
    args: ["--enable-webgpu", "--enable-unsafe-swiftshader", "--use-angle=swiftshader"],
  });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 920 } });
    const cdp = await page.context().newCDPSession(page);
    const base = process.argv[2] || "http://127.0.0.1:4321";
    await page.goto(new URL("/kui/components/textarea/", base).href);
    const preview = page.locator('iframe[data-rendered-story="textarea"]');
    await preview.waitFor();
    await preview.scrollIntoViewIfNeeded();
    await preview.click(); // The multiline editor occupies the center of this story.
    const frame = await (await preview.elementHandle()).contentFrame();
    await frame.waitForFunction(() => document.activeElement instanceof HTMLTextAreaElement &&
      !document.activeElement.readOnly);
    const mirror = frame.locator("textarea");
    const expectText = async text => {
      // A paint and the subsequent mirror sync must agree with the browser's commit.
      await frame.evaluate(() => new Promise(done =>
        requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(done, 0)))));
      assert.equal(await mirror.inputValue(), text);
    };
    const mark = text => cdp.send("Input.imeSetComposition", {
      text, selectionStart: text.length, selectionEnd: text.length,
    });
    await page.keyboard.press(process.platform === "darwin" ? "Meta+a" : "Control+a");
    await page.keyboard.press("Backspace");
    await expectText("");

    // Reproduce a hardware keydown before the browser flags the IME session.
    // A composition-only test misses the extra unmarked 'n' in GPUI's buffer.
    const firstKey = { key: "n", code: "KeyN", windowsVirtualKeyCode: 78 };
    await cdp.send("Input.dispatchKeyEvent", { type: "rawKeyDown", ...firstKey });
    await mark("n");
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", ...firstKey });
    await mark("ni");
    await cdp.send("Input.insertText", { text: "你" });
    await expectText("你");
    await mark("n");
    await mark("");
    await expectText("你");
    await page.keyboard.type("nn");
    await expectText("你nn");
  } finally {
    await browser.close();
  }
}

check().catch(error => { console.error(error); process.exitCode = 1; });
