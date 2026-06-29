import { createRouter, createWebHistory } from "vue-router";

const HomeView = {
  template: "",
};

export default createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: HomeView,
    },
  ],
});
