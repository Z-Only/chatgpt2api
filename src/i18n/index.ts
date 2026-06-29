import { createI18n } from "vue-i18n";
import { resolveSystemLocale } from "./locale";

export default createI18n({
  legacy: false,
  locale: resolveSystemLocale(),
  fallbackLocale: "en",
  messages: {
    en: {
      app: {
        title: "ChatGPT2API",
      },
    },
    zh: {
      app: {
        title: "ChatGPT2API",
      },
    },
  },
});
