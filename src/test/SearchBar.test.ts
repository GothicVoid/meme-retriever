import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import SearchBar from "@/components/SearchBar.vue";

describe("SearchBar", () => {
  it("渲染搜索输入框", () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "" } });
    expect(wrapper.find("input").exists()).toBe(true);
  });

  it("挂载统一输入骨架类，便于复用全局设计 token", () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "hello" } });
    expect(wrapper.get(".search-bar").classes()).toContain("ui-input-shell");
    expect(wrapper.get("input").classes()).toContain("ui-input");
    expect(wrapper.get("button").classes()).toContain("ui-input-clear");
  });

  it("支持自定义 placeholder", () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "", placeholder: "搜台词、角色、动作、场景" } });
    expect(wrapper.find("input").attributes("placeholder")).toBe("搜台词、角色、动作、场景");
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

  it("输入框 focus 时向外派发 focus 事件", async () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "" } });
    await wrapper.find("input").trigger("focus");
    expect(wrapper.emitted("focus")).toBeTruthy();
  });

  it("输入框 blur 时向外派发 blur 事件", async () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "" } });
    await wrapper.find("input").trigger("blur");
    expect(wrapper.emitted("blur")).toBeTruthy();
  });

  it("组合输入期间不派发 update:modelValue，结束后再派发最终值", async () => {
    const wrapper = mount(SearchBar, { props: { modelValue: "" } });
    const input = wrapper.find("input");

    await input.trigger("compositionstart");
    await input.setValue("a");
    expect(wrapper.emitted("update:modelValue")).toBeFalsy();

    await input.setValue("阿");
    expect(wrapper.emitted("update:modelValue")).toBeFalsy();

    await input.trigger("compositionend");
    expect(wrapper.emitted("update:modelValue")).toBeTruthy();
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["阿"]);
  });
});
