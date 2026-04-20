import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type WindowMode = "sidebar" | "expanded";

export const useSettingsStore = defineStore("settings", () => {
  const devDebugMode = ref(false);
  const startupWindowMode = ref<WindowMode>("sidebar");
  const currentWindowMode = ref<WindowMode>("sidebar");

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    devDebugMode.value = parsed.devDebugMode ?? parsed.showDebugInfo ?? false;
    const savedWindowMode = parsed.startupWindowMode ?? parsed.windowMode;
    startupWindowMode.value = savedWindowMode === "expanded" ? "expanded" : "sidebar";
    currentWindowMode.value = startupWindowMode.value;
  }

  watch([devDebugMode, startupWindowMode], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        devDebugMode: devDebugMode.value,
        startupWindowMode: startupWindowMode.value,
      })
    );
  });

  load();
  return { devDebugMode, startupWindowMode, currentWindowMode };
});
