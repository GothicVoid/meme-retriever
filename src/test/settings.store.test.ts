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

  it("启动工作态和当前工作态默认都是 sidebar", () => {
    const s = useSettingsStore();
    expect(s.currentWindowMode).toBe("sidebar");
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
    localStorage.setItem("settings", JSON.stringify({ legacyField: true }));
    const s = useSettingsStore();
    expect(s.devDebugMode).toBe(false);
  });

  it("currentWindowMode 为运行时状态，不持久化到 localStorage", async () => {
    const s = useSettingsStore();
    s.currentWindowMode = "expanded";
    await nextTick();

    expect(localStorage.getItem("settings")).toBeNull();
  });

  it("忽略历史 startupWindowMode 字段，始终以 sidebar 初始化当前工作态", () => {
    localStorage.setItem("settings", JSON.stringify({ startupWindowMode: "expanded" }));
    const s = useSettingsStore();
    expect(s.currentWindowMode).toBe("sidebar");
  });

  it("忽略历史 windowMode 字段", () => {
    localStorage.setItem("settings", JSON.stringify({ windowMode: "expanded" }));
    const s = useSettingsStore();
    expect(s.currentWindowMode).toBe("sidebar");
  });
});
