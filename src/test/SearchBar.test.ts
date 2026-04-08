import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import SearchBar from "@/components/SearchBar.vue";

describe("SearchBar", () => {
  it("渲染搜索输入框", () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "" } });
    expect(wrapper.find("input").exists()).toBe(true);
  });

  it("有值时显示清除按钮", () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "hello" } });
    expect(wrapper.find("button").exists()).toBe(true);
  });

  it("无值时不显示清除按钮", () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "" } });
    expect(wrapper.find("button").exists()).toBe(false);
  });

  it("Ctrl+F 聚焦搜索框", async () => {
    const wrapper = mount(SearchBar, {
      props: { modelValue: "" },
      attachTo: document.body,
    });
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "f", ctrlKey: true, cancelable: true }));
    await wrapper.vm.$nextTick();
    expect(document.activeElement).toBe(wrapper.find("input").element);
    wrapper.unmount();
  });

  it("Meta+F 聚焦搜索框", async () => {
    const wrapper = mount(SearchBar, {
      props: { modelValue: "" },
      attachTo: document.body,
    });
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "f", metaKey: true, cancelable: true }));
    await wrapper.vm.$nextTick();
    expect(document.activeElement).toBe(wrapper.find("input").element);
    wrapper.unmount();
  });

  it("Ctrl+F 阻止默认行为", async () => {
    const wrapper = mount(SearchBar, {
      props: { modelValue: "" },
      attachTo: document.body,
    });
    const event = new KeyboardEvent("keydown", { key: "f", ctrlKey: true, cancelable: true });
    const spy = vi.spyOn(event, "preventDefault");
    window.dispatchEvent(event);
    await wrapper.vm.$nextTick();
    expect(spy).toHaveBeenCalled();
    wrapper.unmount();
  });
});
