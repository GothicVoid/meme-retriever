import { defineStore } from "pinia";
import { ref, watch } from "vue";

export const useSettingsStore = defineStore("settings", () => {
  const defaultLimit = ref(9);
  const devDebugMode = ref(false);

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    defaultLimit.value = parsed.defaultLimit ?? 9;
    devDebugMode.value = parsed.devDebugMode ?? parsed.showDebugInfo ?? false;
  }

  watch([defaultLimit, devDebugMode], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        defaultLimit: defaultLimit.value,
        devDebugMode: devDebugMode.value,
      })
    );
  });

  load();
  return { defaultLimit, devDebugMode };
});
