import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import ImageCard from "@/components/ImageCard.vue";
import Toast from "@/components/Toast.vue";
import type { SearchResult } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

const copyImageMock = vi.fn();
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: copyImageMock }),
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
    copyImageMock.mockReset();
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

  it("selectable=true 时渲染 checkbox", () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false, selectable: true, selected: false },
    });
    expect(wrapper.find("input[type='checkbox']").exists()).toBe(true);
  });

  it("selectable 未传时不渲染 checkbox", () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
    });
    expect(wrapper.find("input[type='checkbox']").exists()).toBe(false);
  });

  it("点击 checkbox 触发 select 事件", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false, selectable: true, selected: false },
    });
    await wrapper.find("input[type='checkbox']").trigger("change");
    expect(wrapper.emitted("select")).toBeTruthy();
    expect(wrapper.emitted("select")![0]).toEqual(["uuid-1"]);
  });

  it("单击图片时复制到剪贴板并显示成功提示", async () => {
    copyImageMock.mockResolvedValue(undefined);
    const wrapper = mount({
      components: { ImageCard, Toast },
      template: '<div><ImageCard :image="image" :show-debug-info="false" /><Toast /></div>',
      data: () => ({ image: mockImage }),
    }, { attachTo: document.body });

    await wrapper.find(".image-card").trigger("click");
    expect(copyImageMock).toHaveBeenCalledWith("uuid-1");

    const toast = document.body.querySelector(".toast");
    expect(toast?.textContent).toContain("已复制");

    wrapper.unmount();
  });

  it("复制失败时显示失败提示", async () => {
    copyImageMock.mockRejectedValue(new Error("copy failed"));
    const wrapper = mount({
      components: { ImageCard, Toast },
      template: '<div><ImageCard :image="image" :show-debug-info="false" /><Toast /></div>',
      data: () => ({ image: mockImage }),
    }, { attachTo: document.body });

    await wrapper.find(".image-card").trigger("click");

    const toast = document.body.querySelector(".toast.error");
    expect(toast?.textContent).toContain("复制失败");

    wrapper.unmount();
  });
});
