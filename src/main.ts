import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import App from "./App.vue";
import "./assets/ui.css";
import SearchView from "./views/SearchView.vue";
import LibraryView from "./views/LibraryView.vue";
import PrivateRoleLibraryView from "./views/PrivateRoleLibraryView.vue";

const routes: RouteRecordRaw[] = [
  { path: "/", component: SearchView },
  { path: "/library", component: LibraryView },
];

if (import.meta.env.DEV) {
  routes.push({
    path: "/private-role-maintenance",
    alias: "/kb-maintenance",
    component: PrivateRoleLibraryView,
  });
}

const router = createRouter({
  history: createWebHistory(),
  routes,
});

createApp(App).use(createPinia()).use(router).mount("#app");
