import { defaultLocale, languages, ui, type Locale, type UiKey } from "./ui";

const base = import.meta.env.BASE_URL.replace(/\/$/, "");

/** A site-root path mounted at the deployment's base (including on Pages). */
export function sitePath(path: string): string {
  return base + (path.startsWith("/") ? path : "/" + path);
}

function routePath(pathname: string): string {
  if (base && (pathname === base || pathname.startsWith(base + "/"))) {
    return pathname.slice(base.length) || "/";
  }
  return pathname;
}

export function isLocale(value: string): value is Locale {
  return value in languages;
}

/** Locale from URL: `/en/...` → en, otherwise zh (default). */
export function getLocale(pathname: string): Locale {
  const seg = routePath(pathname).split("/").filter(Boolean)[0];
  if (seg && isLocale(seg) && seg !== defaultLocale) return seg;
  return defaultLocale;
}

/** Strip locale prefix for matching routes. */
export function stripLocale(pathname: string): string {
  pathname = routePath(pathname);
  const parts = pathname.split("/").filter(Boolean);
  if (parts[0] && isLocale(parts[0]) && parts[0] !== defaultLocale) {
    const rest = parts.slice(1).join("/");
    return rest ? `/${rest}` : "/";
  }
  return pathname.endsWith("/") && pathname !== "/" ? pathname.slice(0, -1) || "/" : pathname;
}

/** Build a localized href. Default zh has no prefix. */
export function localePath(locale: Locale, path: string): string {
  const clean = path.startsWith("/") ? path : `/${path}`;
  if (locale === defaultLocale) return sitePath(clean);
  return sitePath(clean === "/" ? `/${locale}` : `/${locale}${clean}`);
}

export function useTranslations(locale: Locale) {
  return function t(key: UiKey): string {
    return ui[locale][key] ?? ui[defaultLocale][key] ?? key;
  };
}

export function alternateLocale(locale: Locale): Locale {
  return locale === "zh" ? "en" : "zh";
}
