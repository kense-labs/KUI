import { defaultLocale, languages, ui, type Locale, type UiKey } from "./ui";

export function isLocale(value: string): value is Locale {
  return value in languages;
}

/** Locale from URL: `/en/...` → en, otherwise zh (default). */
export function getLocale(pathname: string): Locale {
  const seg = pathname.split("/").filter(Boolean)[0];
  if (seg && isLocale(seg) && seg !== defaultLocale) return seg;
  return defaultLocale;
}

/** Strip locale prefix for matching routes. */
export function stripLocale(pathname: string): string {
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
  if (locale === defaultLocale) return clean === "" ? "/" : clean;
  if (clean === "/") return `/${locale}`;
  return `/${locale}${clean}`;
}

export function useTranslations(locale: Locale) {
  return function t(key: UiKey): string {
    return ui[locale][key] ?? ui[defaultLocale][key] ?? key;
  };
}

export function alternateLocale(locale: Locale): Locale {
  return locale === "zh" ? "en" : "zh";
}
