<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/app";

const app = useAppStore();
const { t } = useI18n();
const efforts = ["low", "medium", "high", "xhigh"];
const summaries = ["auto", "detailed"];
const verbosity = ["low", "medium", "high"];
const themes = ["system", "light", "dark"];
const locales = ["system", "en", "zh"];
</script>

<template>
  <section class="panel">
    <h2>{{ t("model.title") }}</h2>
    <div class="form-grid">
      <label>
        <span>{{ t("model.defaultModel") }}</span>
        <input v-model="app.config.api.default_model" />
      </label>
      <label>
        <span>{{ t("model.reasoning") }}</span>
        <select v-model="app.config.reasoning.effort">
          <option v-for="effort in efforts" :key="effort" :value="effort">
            {{ t(`option.${effort}`) }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ t("model.summary") }}</span>
        <select v-model="app.config.reasoning.summary">
          <option v-for="summary in summaries" :key="summary" :value="summary">
            {{ t(`option.${summary}`) }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ t("model.textVerbosity") }}</span>
        <select v-model="app.config.text.verbosity">
          <option v-for="level in verbosity" :key="level" :value="level">
            {{ t(`option.${level}`) }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ t("model.theme") }}</span>
        <select v-model="app.config.ui.theme">
          <option v-for="theme in themes" :key="theme" :value="theme">
            {{ t(`option.${theme}`) }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ t("model.locale") }}</span>
        <select v-model="app.config.ui.locale">
          <option v-for="locale in locales" :key="locale" :value="locale">
            {{ t(`option.${locale}`) }}
          </option>
        </select>
      </label>
    </div>
    <button type="button" @click="app.save">{{ t("common.save") }}</button>
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
  margin: 0 0 16px;
  font-size: 1rem;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

label {
  display: grid;
  gap: 6px;
  color: var(--color-muted);
  font-size: 0.8125rem;
}

input,
select {
  min-height: 36px;
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 0 10px;
  color: var(--color-fg);
  background: var(--color-surface-subtle);
}

button {
  min-height: 36px;
  margin-top: 16px;
  padding: 0 14px;
  border: 1px solid var(--color-accent);
  border-radius: 6px;
  color: #fff;
  background: var(--color-accent);
  cursor: pointer;
}
</style>
