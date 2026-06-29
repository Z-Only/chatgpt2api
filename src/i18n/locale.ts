export type AppLocale = "en" | "zh";

export function resolveSystemLocale(languages?: readonly string[]): AppLocale {
  const source = languages ?? (typeof navigator === "undefined" ? ["en"] : navigator.languages);
  const first = source[0]?.toLowerCase() ?? "en";
  return first.startsWith("zh") ? "zh" : "en";
}
