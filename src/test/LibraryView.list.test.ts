import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import LibraryView from "@/views/LibraryView.vue";
import type { ImageMeta } from "@/stores/library";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  confirm: vi.fn(),
}));

vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const mockInvoke = vi.mocked(invoke);

function makeImages(count: number): ImageMeta[] {
  return Array.from({ length: count }, (_, index) => ({
    id: `img-${index}`,
    filePath: `/library/images/img-${index}.jpg`,
    fileName: `img-${index}.jpg`,
    thumbnailPath: `/library/thumbs/img-${index}.jpg`,
    width: 800,
    height: 600,
    addedAt: 1_700_000_000 + count - index,
    useCount: 0,
    tags: [],
  }));
}

function setScrollMetrics(el: Element, metrics: { clientHeight: number; scrollHeight: number; scrollTop: number }) {
  Object.defineProperty(el, "clientHeight", { value: metrics.clientHeight, configurable: true });
  Object.defineProperty(el, "scrollHeight", { value: metrics.scrollHeight, configurable: true });
  Object.defineProperty(el, "scrollTop", { value: metrics.scrollTop, writable: true, configurable: true });
}

describe("LibraryView 图片列表展示", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    HTMLElement.prototype.scrollTo = vi.fn();
  });

  it("首屏显示总数且只加载 15 张图片", async () => {
    const images = makeImages(15);
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 20;
      if (cmd === "get_images" && args?.page === 0) return images;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("共 20 张");
    expect(wrapper.findAll(".image-card")).toHaveLength(15);
    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 0 });

    wrapper.unmount();
  });

  it("顶部显示按路径引用图片的使用提示", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 1;
      if (cmd === "get_images" && args?.page === 0) return makeImages(1);
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find(".usage-notice").exists()).toBe(true);
    expect(wrapper.text()).toContain("图库按原文件路径引用");
    expect(wrapper.text()).toContain("影响复制和定位");

    wrapper.unmount();
  });

  it("滚动到底部时自动加载下一页并显示已到底部提示", async () => {
    const page0 = makeImages(15);
    const page1 = makeImages(5).map((image, index) => ({ ...image, id: `img-next-${index}` }));

    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 20;
      if (cmd === "get_images" && args?.page === 0) return page0;
      if (cmd === "get_images" && args?.page === 1) return page1;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    const scroller = wrapper.get(".gallery-scroll").element;
    setScrollMetrics(scroller, { clientHeight: 400, scrollHeight: 1200, scrollTop: 760 });
    await wrapper.get(".gallery-scroll").trigger("scroll");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 1 });
    expect(wrapper.findAll(".image-card")).toHaveLength(20);
    expect(wrapper.text()).toContain("已显示全部图片");

    wrapper.unmount();
  });

  it("图库为空时显示空状态提示", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });

    const wrapper = mount(LibraryView);
    await flushPromises();

    expect(wrapper.text()).toContain("图库为空，请先添加图片");
  });

  it("初始加载失败时显示重试按钮并可重新加载", async () => {
    mockInvoke
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(2)
      .mockResolvedValueOnce(makeImages(2));

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("加载失败，请重试");

    await wrapper.get("[data-action='retry-load']").trigger("click");
    await flushPromises();

    expect(wrapper.findAll(".image-card")).toHaveLength(2);
    wrapper.unmount();
  });

  it("滚动超过一页后显示回到顶部按钮，并平滑滚动到顶部", async () => {
    const images = makeImages(15);
    const scrollToMock = vi.fn();
    HTMLElement.prototype.scrollTo = scrollToMock;

    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 30;
      if (cmd === "get_images" && args?.page === 0) return images;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    const scroller = wrapper.get(".gallery-scroll").element;
    setScrollMetrics(scroller, { clientHeight: 400, scrollHeight: 1200, scrollTop: 500 });
    await wrapper.get(".gallery-scroll").trigger("scroll");
    await wrapper.vm.$nextTick();

    expect(wrapper.find("[data-action='back-to-top']").exists()).toBe(true);

    await wrapper.get("[data-action='back-to-top']").trigger("click");
    expect(scrollToMock).toHaveBeenCalledWith({ top: 0, behavior: "smooth" });

    wrapper.unmount();
  });
});
