import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import SearchView from "./views/SearchView.vue";
import LibraryView from "./views/LibraryView.vue";
import SettingsView from "./views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", component: SearchView },
    { path: "/library", component: LibraryView },
    { path: "/settings", component: SettingsView },
  ],
});

createApp(App).use(createPinia()).use(router).mount("#app");
