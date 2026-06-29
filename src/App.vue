<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <main class="app-shell">
    <h1>{{ t("app.title") }}</h1>
  </main>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
  display: grid;
  place-items: center;
  margin: 0;
  color: #171717;
  background: #fff;
}

h1 {
  margin: 0;
  font-size: 2rem;
  font-weight: 650;
}
</style>
