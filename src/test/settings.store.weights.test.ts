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
    s.devDebugMode = true;
    await nextTick();
    const saved = JSON.parse(localStorage.getItem("settings")!);
    expect(saved.devDebugMode).toBe(true);
    expect(saved.showDebugInfo).toBeUndefined();
    expect(saved.w1).toBeUndefined();
    expect(saved.w2).toBeUndefined();
    expect(saved.w3).toBeUndefined();
  });

  it("从 localStorage 加载历史权重字段时忽略它们", () => {
    localStorage.setItem(
      "settings",
      JSON.stringify({ showDebugInfo: true, w1: 0.6, w2: 0.2, w3: 0.2, defaultLimit: 9 })
    );
    const s = useSettingsStore() as unknown as Record<string, unknown>;
    expect(s.devDebugMode).toBe(true);
    expect("defaultLimit" in s).toBe(false);
    expect("w1" in s).toBe(false);
    expect("w2" in s).toBe(false);
    expect("w3" in s).toBe(false);
  });
});
