import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";
import { useSettingsStore } from "@/stores/settings";

describe("useSettingsStore — 搜索权重已移除", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("不再暴露 w1/w2/w3 和 normalizedWeights", () => {
    const s = useSettingsStore() as unknown as Record<string, unknown>;
    expect("w1" in s).toBe(false);
    expect("w2" in s).toBe(false);
    expect("w3" in s).toBe(false);
    expect("normalizedWeights" in s).toBe(false);
  });

  it("持久化 settings 时不写入历史权重字段", async () => {
    const s = useSettingsStore();
    s.showDebugInfo = true;
    await nextTick();
    const saved = JSON.parse(localStorage.getItem("settings")!);
    expect(saved.showDebugInfo).toBe(true);
    expect(saved.w1).toBeUndefined();
    expect(saved.w2).toBeUndefined();
    expect(saved.w3).toBeUndefined();
  });

  it("从 localStorage 加载历史权重字段时忽略它们", () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({ defaultLimit: 9, showDebugInfo: true, w1: 0.6, w2: 0.2, w3: 0.2 })
    );
    const s = useSettingsStore() as unknown as Record<string, unknown>;
    expect(s.showDebugInfo).toBe(true);
    expect("w1" in s).toBe(false);
    expect("w2" in s).toBe(false);
    expect("w3" in s).toBe(false);
  });
});
