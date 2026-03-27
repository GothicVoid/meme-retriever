import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import ImageCard from "@/components/ImageCard.vue";
import type { SearchResult } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const mockImage: SearchResult = {
  id: "uuid-1",
  filePath: "/library/images/uuid-1.jpg",
  thumbnailPath: "/library/thumbs/uuid-1.jpg",
  score: 0.9,
  tags: ["搞笑"],
  debugInfo: null,
};

describe("ImageCard", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("渲染缩略图", () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    const img = wrapper.find("img");
    expect(img.exists()).toBe(true);
    expect(img.attributes("src")).toContain("uuid-1");
  });

  it("右键点击显示上下文菜单", async () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    await wrapper.trigger("contextmenu");
    expect(wrapper.find(".context-menu").exists()).toBe(true);
  });

  it("上下文菜单包含删除选项", async () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    await wrapper.trigger("contextmenu");
    const menu = wrapper.find(".context-menu");
    expect(menu.text()).toContain("删除");
  });

  it("点击删除菜单项触发 delete 事件", async () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    await wrapper.trigger("contextmenu");
    const deleteBtn = wrapper.find("[data-action='delete']");
    await deleteBtn.trigger("click");
    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["uuid-1"]);
  });

  it("点击其他区域关闭上下文菜单", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(wrapper.find(".context-menu").exists()).toBe(true);

    await document.dispatchEvent(new MouseEvent("click"));
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".context-menu").exists()).toBe(false);

    wrapper.unmount();
  });

  it("showDebugInfo=false 时不显示调试叠层", () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    expect(wrapper.find(".debug-overlay").exists()).toBe(false);
  });

  it("showDebugInfo=true 且 debugInfo 为 null 时不显示叠层", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...mockImage, debugInfo: null }, showDebugInfo: true },
    });
    expect(wrapper.find(".debug-overlay").exists()).toBe(false);
  });

  it("showDebugInfo=true 且有 debugInfo 时显示叠层", () => {
    const image: SearchResult = {
      ...mockImage,
      debugInfo: { semScore: 0.8, kwScore: 0.3, tagHit: false, semWeight: 0.3, kwWeight: 0.4, relevance: 0.24, popularity: 0.5 },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: true } });
    const overlay = wrapper.find(".debug-overlay");
    expect(overlay.exists()).toBe(true);
    expect(overlay.text()).toContain("80");
    expect(overlay.text()).toContain("30");
  });

  it("标签命中时显示标签命中标记", () => {
    const image: SearchResult = {
      ...mockImage,
      debugInfo: { semScore: 0.5, kwScore: 0.9, tagHit: true, semWeight: 0.3, kwWeight: 0.4, relevance: 0.36, popularity: 0.8 },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: true } });
    expect(wrapper.find(".debug-overlay").text()).toContain("标签命中");
  });
});
