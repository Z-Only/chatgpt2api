import { createI18n } from "vue-i18n";
import en from "./en";
import { resolveSystemLocale } from "./locale";
import zh from "./zh";

export default createI18n({
  legacy: false,
  locale: resolveSystemLocale(),
  fallbackLocale: "en",
  messages: { en, zh },
});
