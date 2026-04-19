import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type WindowMode = "sidebar" | "expanded";

export const useSettingsStore = defineStore("settings", () => {
  const defaultLimit = ref(9);
  const devDebugMode = ref(false);
  const windowMode = ref<WindowMode>("sidebar");

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    defaultLimit.value = parsed.defaultLimit ?? 9;
    devDebugMode.value = parsed.devDebugMode ?? parsed.showDebugInfo ?? false;
    windowMode.value = parsed.windowMode === "expanded" ? "expanded" : "sidebar";
  }

  watch([defaultLimit, devDebugMode, windowMode], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        defaultLimit: defaultLimit.value,
        devDebugMode: devDebugMode.value,
        windowMode: windowMode.value,
      })
    );
  });

  load();
  return { defaultLimit, devDebugMode, windowMode };
});
