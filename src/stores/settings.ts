import { defineStore } from "pinia";
import { ref, watch } from "vue";

export const useSettingsStore = defineStore("settings", () => {
  const defaultLimit = ref(9);
  const showDebugInfo = ref(false);

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    defaultLimit.value = parsed.defaultLimit ?? 9;
    showDebugInfo.value = parsed.showDebugInfo ?? false;
  }

  watch([defaultLimit, showDebugInfo], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        defaultLimit: defaultLimit.value,
        showDebugInfo: showDebugInfo.value,
      })
    );
  });

  load();
  return { defaultLimit, showDebugInfo };
});
