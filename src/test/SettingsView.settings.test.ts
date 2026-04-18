import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import SettingsView from "@/views/SettingsView.vue";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: vi.fn(),
}));

describe("SettingsView 基础设置", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("不再显示搜索权重调节区", () => {
    const wrapper = mount(SettingsView);
    expect(wrapper.text()).not.toContain("搜索权重调节");
    expect(wrapper.text()).not.toContain("关键词 (w1)");
    expect(wrapper.text()).not.toContain("OCR (w2)");
    expect(wrapper.text()).not.toContain("CLIP (w3)");
    expect(wrapper.text()).toContain("开发调试模式");
    expect(wrapper.text()).not.toContain("显示调试信息");
    expect(wrapper.find(".weights-section").exists()).toBe(false);
  });
});
