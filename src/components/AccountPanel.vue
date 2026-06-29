<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/app";

const app = useAppStore();
const { t } = useI18n();
</script>

<template>
  <section class="panel">
    <div class="panel-header">
      <h2>{{ t("account.title") }}</h2>
      <span class="status">{{ app.account.email ?? t("account.signedOut") }}</span>
    </div>
    <div class="actions">
      <button type="button" @click="app.login">{{ t("account.login") }}</button>
      <button type="button" class="secondary" @click="app.logout">{{ t("account.logout") }}</button>
    </div>
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

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

h2 {
  margin: 0;
  font-size: 1rem;
}

.status {
  color: var(--color-muted);
  font-size: 0.875rem;
}

.actions {
  display: flex;
  gap: 10px;
  margin-top: 16px;
}

button {
  min-height: 36px;
  padding: 0 14px;
  border: 1px solid var(--color-accent);
  border-radius: 6px;
  color: #fff;
  background: var(--color-accent);
  cursor: pointer;
}

.secondary {
  color: var(--color-fg);
  background: transparent;
  border-color: var(--color-border);
}
</style>
