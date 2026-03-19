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
};

describe("ImageCard", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("渲染缩略图", () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage } });
    const img = wrapper.find("img");
    expect(img.exists()).toBe(true);
    expect(img.attributes("src")).toContain("uuid-1");
  });

  it("右键点击显示上下文菜单", async () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage } });
    await wrapper.trigger("contextmenu");
    expect(wrapper.find(".context-menu").exists()).toBe(true);
  });

  it("上下文菜单包含删除选项", async () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage } });
    await wrapper.trigger("contextmenu");
    const menu = wrapper.find(".context-menu");
    expect(menu.text()).toContain("删除");
  });

  it("点击删除菜单项触发 delete 事件", async () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage } });
    await wrapper.trigger("contextmenu");
    const deleteBtn = wrapper.find("[data-action='delete']");
    await deleteBtn.trigger("click");
    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["uuid-1"]);
  });

  it("点击其他区域关闭上下文菜单", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(wrapper.find(".context-menu").exists()).toBe(true);

    await document.dispatchEvent(new MouseEvent("click"));
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".context-menu").exists()).toBe(false);

    wrapper.unmount();
  });
});
