import { defineStore } from "pinia";
import { ref, watch } from "vue";

export const useSettingsStore = defineStore("settings", () => {
  const defaultLimit = ref(9);
  const deleteOriginalFile = ref(false);
  const libraryPath = ref("");
  const showDebugInfo = ref(false);

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    defaultLimit.value = parsed.defaultLimit ?? 9;
    deleteOriginalFile.value = parsed.deleteOriginalFile ?? false;
    libraryPath.value = parsed.libraryPath ?? "";
    showDebugInfo.value = parsed.showDebugInfo ?? false;
  }

  watch([defaultLimit, deleteOriginalFile, libraryPath, showDebugInfo], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        defaultLimit: defaultLimit.value,
        deleteOriginalFile: deleteOriginalFile.value,
        libraryPath: libraryPath.value,
        showDebugInfo: showDebugInfo.value,
      })
    );
  });

  load();
  return { defaultLimit, deleteOriginalFile, libraryPath, showDebugInfo };
});
