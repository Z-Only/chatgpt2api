<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/app";

const app = useAppStore();
const { t } = useI18n();
</script>

<template>
  <section class="panel">
    <div class="panel-header">
      <h2>{{ t("server.title") }}</h2>
      <strong :class="app.server.running ? 'running' : 'stopped'">
        {{ app.server.running ? t("server.running") : t("server.stopped") }}
      </strong>
    </div>
    <div class="form-grid">
      <label>
        <span>{{ t("server.host") }}</span>
        <input v-model="app.config.server.host" />
      </label>
      <label>
        <span>{{ t("server.port") }}</span>
        <input v-model.number="app.config.server.port" type="number" min="1" max="65535" />
      </label>
      <label class="wide">
        <span>{{ t("server.baseUrl") }}</span>
        <input :value="app.server.url" readonly />
      </label>
    </div>
    <div class="actions">
      <button type="button" @click="app.start">{{ t("server.start") }}</button>
      <button type="button" class="secondary" @click="app.stop">{{ t("server.stop") }}</button>
      <button type="button" class="secondary" @click="app.save">{{ t("common.save") }}</button>
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

.panel-header,
.actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.panel-header {
  justify-content: space-between;
}

h2 {
  margin: 0;
  font-size: 1rem;
}

strong {
  font-size: 0.875rem;
}

.running {
  color: var(--color-success);
}

.stopped {
  color: var(--color-muted);
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 16px;
}

.wide {
  grid-column: 1 / -1;
}

label {
  display: grid;
  gap: 6px;
  color: var(--color-muted);
  font-size: 0.8125rem;
}

input {
  min-height: 36px;
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 0 10px;
  color: var(--color-fg);
  background: var(--color-surface-subtle);
}

.actions {
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
