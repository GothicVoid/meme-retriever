import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type WindowMode = "sidebar" | "expanded";

export const useSettingsStore = defineStore("settings", () => {
  const defaultLimit = ref(9);
  const devDebugMode = ref(false);
  const startupWindowMode = ref<WindowMode>("sidebar");
  const currentWindowMode = ref<WindowMode>("sidebar");

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    defaultLimit.value = parsed.defaultLimit ?? 9;
    devDebugMode.value = parsed.devDebugMode ?? parsed.showDebugInfo ?? false;
    const savedWindowMode = parsed.startupWindowMode ?? parsed.windowMode;
    startupWindowMode.value = savedWindowMode === "expanded" ? "expanded" : "sidebar";
    currentWindowMode.value = startupWindowMode.value;
  }

  watch([defaultLimit, devDebugMode, startupWindowMode], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        defaultLimit: defaultLimit.value,
        devDebugMode: devDebugMode.value,
        startupWindowMode: startupWindowMode.value,
      })
    );
  });

  load();
  return { defaultLimit, devDebugMode, startupWindowMode, currentWindowMode };
});
