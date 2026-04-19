import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";
import { useSettingsStore } from "@/stores/settings";

describe("useSettingsStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("devDebugMode 默认为 false", () => {
    const s = useSettingsStore();
    expect(s.devDebugMode).toBe(false);
  });

  it("窗口模式默认是 sidebar", () => {
    const s = useSettingsStore();
    expect(s.windowMode).toBe("sidebar");
  });

  it("devDebugMode 修改后持久化到 localStorage", async () => {
    const s = useSettingsStore();
    s.devDebugMode = true;
    await nextTick();
    const saved = JSON.parse(localStorage.getItem("settings")!);
    expect(saved.devDebugMode).toBe(true);
  });

  it("从 localStorage 加载 devDebugMode=true", () => {
    localStorage.setItem("settings", JSON.stringify({ devDebugMode: true }));
    const s = useSettingsStore();
    expect(s.devDebugMode).toBe(true);
  });

  it("兼容读取历史 showDebugInfo=true", () => {
    localStorage.setItem("settings", JSON.stringify({ showDebugInfo: true }));
    const s = useSettingsStore();
    expect(s.devDebugMode).toBe(true);
  });

  it("devDebugMode 缺失时默认为 false", () => {
    localStorage.setItem("settings", JSON.stringify({ defaultLimit: 9 }));
    const s = useSettingsStore();
    expect(s.devDebugMode).toBe(false);
  });

  it("窗口偏好修改后持久化到 localStorage", async () => {
    const s = useSettingsStore();
    s.windowMode = "expanded";
    await nextTick();

    const saved = JSON.parse(localStorage.getItem("settings")!);
    expect(saved.windowMode).toBe("expanded");
  });

  it("从 localStorage 加载窗口偏好", () => {
    localStorage.setItem("settings", JSON.stringify({ windowMode: "expanded" }));
    const s = useSettingsStore();
    expect(s.windowMode).toBe("expanded");
  });
});
