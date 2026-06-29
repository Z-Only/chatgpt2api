<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/app";

const app = useAppStore();
const { t } = useI18n();
const prompt = ref("");
const sizes = ["auto", "1024x1024", "1024x1536", "1536x1024"];
const quality = ["auto", "low", "medium", "high"];
const backgrounds = ["auto", "transparent", "opaque"];
const formats = ["png", "jpeg", "webp"];
</script>

<template>
  <section class="panel">
    <h2>{{ t("image.title") }}</h2>
    <label class="wide">
      <span>{{ t("image.prompt") }}</span>
      <textarea v-model="prompt" rows="4" />
    </label>
    <div class="form-grid">
      <label>
        <span>{{ t("image.model") }}</span>
        <input v-model="app.config.image.default_model" />
      </label>
      <label>
        <span>{{ t("image.size") }}</span>
        <select v-model="app.config.image.size">
          <option v-for="size in sizes" :key="size" :value="size">{{ size }}</option>
        </select>
      </label>
      <label>
        <span>{{ t("image.quality") }}</span>
        <select v-model="app.config.image.quality">
          <option v-for="level in quality" :key="level" :value="level">
            {{ t(`option.${level}`) }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ t("image.background") }}</span>
        <select v-model="app.config.image.background">
          <option v-for="background in backgrounds" :key="background" :value="background">
            {{ background }}
          </option>
        </select>
      </label>
      <label>
        <span>{{ t("image.outputFormat") }}</span>
        <select v-model="app.config.image.output_format">
          <option v-for="format in formats" :key="format" :value="format">{{ format }}</option>
        </select>
      </label>
    </div>
    <button type="button" @click="app.generateCurrentImage(prompt)">
      {{ t("image.generate") }}
    </button>
    <p v-if="app.error" class="error">{{ t("common.error") }}: {{ app.error }}</p>
    <figure v-if="app.lastGeneratedImage">
      <img :src="`data:image/png;base64,${app.lastGeneratedImage}`" :alt="t('image.preview')" />
      <figcaption>{{ t("image.preview") }}</figcaption>
    </figure>
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
  margin-top: 12px;
}

label {
  display: grid;
  gap: 6px;
  color: var(--color-muted);
  font-size: 0.8125rem;
}

.wide {
  display: grid;
}

input,
select,
textarea {
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 9px 10px;
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

.error {
  color: var(--color-danger);
  font-size: 0.875rem;
}

figure {
  margin: 16px 0 0;
}

img {
  max-width: min(100%, 520px);
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

figcaption {
  margin-top: 8px;
  color: var(--color-muted);
  font-size: 0.8125rem;
}
</style>
