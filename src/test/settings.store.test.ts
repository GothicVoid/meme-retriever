import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";
import { useSettingsStore } from "@/stores/settings";

describe("useSettingsStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("showDebugInfo 默认为 false", () => {
    const s = useSettingsStore();
    expect(s.showDebugInfo).toBe(false);
  });

  it("showDebugInfo 修改后持久化到 localStorage", async () => {
    const s = useSettingsStore();
    s.showDebugInfo = true;
    await nextTick();
    const saved = JSON.parse(localStorage.getItem("settings")!);
    expect(saved.showDebugInfo).toBe(true);
  });

  it("从 localStorage 加载 showDebugInfo=true", () => {
    localStorage.setItem("settings", JSON.stringify({ showDebugInfo: true }));
    const s = useSettingsStore();
    expect(s.showDebugInfo).toBe(true);
  });

  it("showDebugInfo 缺失时默认为 false", () => {
    localStorage.setItem("settings", JSON.stringify({ defaultLimit: 9 }));
    const s = useSettingsStore();
    expect(s.showDebugInfo).toBe(false);
  });
});
