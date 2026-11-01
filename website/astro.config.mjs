import { defineConfig } from "astro/config";
import { readFileSync } from "node:fs";

const site = new URL(process.env.KUI_SITE_URL || "https://kense.ai");
const base = site.pathname.replace(/\/$/, "");
const redirect = (path) => base + path;

export default defineConfig({
  site: site.origin,
  base: site.pathname,
  i18n: {
    defaultLocale: "zh",
    locales: ["zh", "en"],
    routing: {
      prefixDefaultLocale: false,
    },
  },
  server: {
    host: "127.0.0.1",
    port: 4321,
  },
  redirects: {
    "/docs": redirect("/kui/docs/"),
    "/docs/installation": redirect("/kui/docs/installation/"),
    "/components": redirect("/kui/components/button/"),
    "/components/button": redirect("/kui/components/button/"),
    "/ai": redirect("/kui/ai/"),
    "/en/docs": redirect("/en/kui/docs/"),
    "/en/ai": redirect("/en/kui/ai/"),
    "/en/components/button": redirect("/en/kui/components/button/"),
  },
  vite: {
    define: {
      "import.meta.env.PUBLIC_PREVIEW_REVISION": JSON.stringify(
        readFileSync(new URL("./public/stories/revision.txt", import.meta.url), "utf8").trim(),
      ),
    },
    server: {
      fs: {
        allow: [".."],
      },
    },
  },
});
