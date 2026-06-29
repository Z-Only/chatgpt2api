import { defineComponent, h } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import AccountPanel from "../components/AccountPanel.vue";
import ImagePanel from "../components/ImagePanel.vue";
import LogPanel from "../components/LogPanel.vue";
import ModelPanel from "../components/ModelPanel.vue";
import ServerPanel from "../components/ServerPanel.vue";

const HomeView = defineComponent({
  setup: () => () =>
    h("div", { class: "dashboard-grid" }, [h(AccountPanel), h(ServerPanel), h(ModelPanel)]),
});

const SettingsView = defineComponent({
  setup: () => () => h("div", { class: "dashboard-grid" }, [h(ServerPanel), h(ModelPanel)]),
});

export default createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: HomeView,
    },
    {
      path: "/images",
      component: ImagePanel,
    },
    {
      path: "/settings",
      component: SettingsView,
    },
    {
      path: "/logs",
      component: LogPanel,
    },
  ],
});
