import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://kense.ai",
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
    "/docs": "/kui/docs",
    "/docs/installation": "/kui/docs/installation",
    "/components": "/kui/components/button",
    "/components/button": "/kui/components/button",
    "/ai": "/kui/ai",
    "/en/docs": "/en/kui/docs",
    "/en/ai": "/en/kui/ai",
    "/en/components/button": "/en/kui/components/button",
  },
  vite: {
    server: {
      fs: {
        allow: [".."],
      },
    },
  },
});
