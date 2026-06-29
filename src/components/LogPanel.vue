<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/app";

const app = useAppStore();
const { t } = useI18n();
</script>

<template>
  <section class="panel">
    <h2>{{ t("logs.title") }}</h2>
    <p class="path">{{ app.configPath }}</p>
    <ul v-if="app.logs.length">
      <li v-for="entry in app.logs" :key="entry">{{ entry }}</li>
    </ul>
    <p v-else class="empty">{{ t("logs.empty") }}</p>
  </section>
</template>

<style scoped>
.panel {
  padding: 18px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
  box-shadow: var(--shadow-panel);
}

h2 {
  margin: 0 0 12px;
  font-size: 1rem;
}

.path,
.empty,
li {
  color: var(--color-muted);
  font-size: 0.875rem;
}

ul {
  margin: 0;
  padding-left: 20px;
}
</style>
