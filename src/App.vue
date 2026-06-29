<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useI18n } from "vue-i18n";
import { RouterLink, RouterView } from "vue-router";
import { resolveSystemLocale } from "./i18n/locale";
import { useAppStore } from "./stores/app";
import { applyTheme } from "./theme";

const app = useAppStore();
const { locale, t } = useI18n();
const navItems = [
  { to: "/", label: "nav.home" },
  { to: "/images", label: "nav.images" },
  { to: "/settings", label: "nav.settings" },
  { to: "/logs", label: "nav.logs" },
];

onMounted(() => {
  void app.load();
});

watch(
  () => app.config.ui.theme,
  (theme) => applyTheme(theme),
  { immediate: true },
);

watch(
  () => app.config.ui.locale,
  (value) => {
    locale.value = value === "system" ? resolveSystemLocale() : value;
  },
  { immediate: true },
);
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <h1>{{ t("app.title") }}</h1>
      <nav>
        <RouterLink v-for="item in navItems" :key="item.to" :to="item.to">
          {{ t(item.label) }}
        </RouterLink>
      </nav>
    </aside>
    <main class="workspace">
      <header class="topbar">
        <span>{{ app.server.running ? t("server.running") : t("server.stopped") }}</span>
        <strong>{{ app.server.url }}</strong>
      </header>
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
  color: var(--color-fg);
  background: var(--color-bg);
}

h1 {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 700;
}

.sidebar {
  border-right: 1px solid var(--color-border);
  padding: 22px 16px;
  background: var(--color-surface);
}

nav {
  display: grid;
  gap: 6px;
  margin-top: 28px;
}

a {
  padding: 10px 12px;
  border-radius: 6px;
  color: var(--color-muted);
  text-decoration: none;
}

a.router-link-active {
  color: var(--color-fg);
  background: var(--color-surface-subtle);
}

.workspace {
  min-width: 0;
  padding: 22px;
}

.topbar {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
  color: var(--color-muted);
  font-size: 0.875rem;
}

.topbar strong {
  color: var(--color-fg);
  font-weight: 600;
}

@media (width <= 760px) {
  .app-shell {
    grid-template-columns: 1fr;
  }

  .sidebar {
    border-right: 0;
    border-bottom: 1px solid var(--color-border);
  }

  nav {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  a {
    text-align: center;
  }
}
</style>
