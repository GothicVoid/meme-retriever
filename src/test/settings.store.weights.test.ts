import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";
import { useSettingsStore } from "@/stores/settings";

describe("useSettingsStore — 权重配置", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("w1/w2/w3 默认值为 0.3/0.4/0.3", () => {
    const s = useSettingsStore();
    expect(s.w1).toBe(0.3);
    expect(s.w2).toBe(0.4);
    expect(s.w3).toBe(0.3);
  });

  it("normalizedWeights 默认值之和为 1", () => {
    const s = useSettingsStore();
    const { w1, w2, w3 } = s.normalizedWeights;
    expect(w1 + w2 + w3).toBeCloseTo(1, 5);
  });

  it("修改权重后 normalizedWeights 仍归一化", () => {
    const s = useSettingsStore();
    s.w1 = 1;
    s.w2 = 1;
    s.w3 = 0;
    const { w1, w2, w3 } = s.normalizedWeights;
    expect(w1 + w2 + w3).toBeCloseTo(1, 5);
    expect(w1).toBeCloseTo(0.5, 5);
    expect(w2).toBeCloseTo(0.5, 5);
    expect(w3).toBeCloseTo(0, 5);
  });

  it("全为 0 时 normalizedWeights 回退到默认值", () => {
    const s = useSettingsStore();
    s.w1 = 0;
    s.w2 = 0;
    s.w3 = 0;
    const { w1, w2, w3 } = s.normalizedWeights;
    expect(w1).toBe(0.3);
    expect(w2).toBe(0.4);
    expect(w3).toBe(0.3);
  });

  it("权重修改后持久化到 localStorage", async () => {
    const s = useSettingsStore();
    s.w1 = 0.5;
    s.w2 = 0.3;
    s.w3 = 0.2;
    await nextTick();
    const saved = JSON.parse(localStorage.getItem("settings")!);
    expect(saved.w1).toBe(0.5);
    expect(saved.w2).toBe(0.3);
    expect(saved.w3).toBe(0.2);
  });

  it("从 localStorage 加载权重", () => {
    localStorage.setItem("settings", JSON.stringify({ w1: 0.6, w2: 0.2, w3: 0.2 }));
    const s = useSettingsStore();
    expect(s.w1).toBe(0.6);
    expect(s.w2).toBe(0.2);
    expect(s.w3).toBe(0.2);
  });

  it("localStorage 缺少权重字段时使用默认值", () => {
    localStorage.setItem("settings", JSON.stringify({ defaultLimit: 9 }));
    const s = useSettingsStore();
    expect(s.w1).toBe(0.3);
    expect(s.w2).toBe(0.4);
    expect(s.w3).toBe(0.3);
  });
});
