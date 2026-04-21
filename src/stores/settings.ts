import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type WindowMode = "sidebar" | "expanded";

export const useSettingsStore = defineStore("settings", () => {
  const devDebugMode = ref(false);
  const currentWindowMode = ref<WindowMode>("sidebar");

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    devDebugMode.value = parsed.devDebugMode ?? parsed.showDebugInfo ?? false;
  }

  watch(devDebugMode, () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        devDebugMode: devDebugMode.value,
      })
    );
  });

  load();
  return { devDebugMode, currentWindowMode };
});
