import { defineStore } from "pinia";
import { ref, watch, computed } from "vue";

export const useSettingsStore = defineStore("settings", () => {
  const defaultLimit = ref(9);
  const showDebugInfo = ref(false);
  // PRD §5.6: 搜索权重（w1=关键词, w2=OCR, w3=CLIP）
  const w1 = ref(0.3);
  const w2 = ref(0.4);
  const w3 = ref(0.3);

  // 归一化后的权重（确保 w1+w2+w3=1）
  const normalizedWeights = computed(() => {
    const sum = w1.value + w2.value + w3.value;
    if (sum === 0) return { w1: 0.3, w2: 0.4, w3: 0.3 };
    return { w1: w1.value / sum, w2: w2.value / sum, w3: w3.value / sum };
  });

  function load() {
    const raw = localStorage.getItem("settings");
    if (!raw) return;
    const parsed = JSON.parse(raw);
    defaultLimit.value = parsed.defaultLimit ?? 9;
    showDebugInfo.value = parsed.showDebugInfo ?? false;
    w1.value = parsed.w1 ?? 0.3;
    w2.value = parsed.w2 ?? 0.4;
    w3.value = parsed.w3 ?? 0.3;
  }

  watch([defaultLimit, showDebugInfo, w1, w2, w3], () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({
        defaultLimit: defaultLimit.value,
        showDebugInfo: showDebugInfo.value,
        w1: w1.value,
        w2: w2.value,
        w3: w3.value,
      })
    );
  });

  load();
  return { defaultLimit, showDebugInfo, w1, w2, w3, normalizedWeights };
});
