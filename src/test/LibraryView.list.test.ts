import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import LibraryView from "@/views/LibraryView.vue";
import type { ImageMeta } from "@/stores/library";

function pageOf(args: unknown): number | undefined {
  if (typeof args !== "object" || args === null || Array.isArray(args)) {
    return undefined;
  }
  if (args instanceof ArrayBuffer || args instanceof Uint8Array) {
    return undefined;
  }
  return "page" in args ? (args as { page?: number }).page : undefined;
}

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

function setScrollMetrics(
  el: Element,
  metrics: { clientHeight: number; scrollHeight: number; scrollTop: number }
) {
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

  it("首屏显示总数且按单页上限加载图片", async () => {
    const images = makeImages(24);
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 30;
      if (cmd === "get_images" && pageOf(args) === 0) return images;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("共 30 张");
    expect(wrapper.findAll(".image-card")).toHaveLength(24);
    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 0 });

    wrapper.unmount();
  });

  it("顶部显示简短的路径失效提示", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 1;
      if (cmd === "get_images" && pageOf(args) === 0) return makeImages(1);
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find(".usage-notice").exists()).toBe(true);
    expect(wrapper.text()).toContain("原文件移动、重命名或删除后会失效");

    wrapper.unmount();
  });

  it("滚动到底部时自动加载下一页并显示已到底部提示", async () => {
    const page0 = makeImages(24);
    const page1 = makeImages(6).map((image, index) => ({ ...image, id: `img-next-${index}` }));

    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 30;
      if (cmd === "get_images" && pageOf(args) === 0) return page0;
      if (cmd === "get_images" && pageOf(args) === 1) return page1;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    const scroller = wrapper.get(".gallery-scroll").element;
    setScrollMetrics(scroller, { clientHeight: 400, scrollHeight: 1200, scrollTop: 760 });
    await wrapper.get(".gallery-scroll").trigger("scroll");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 1 });
    expect(wrapper.findAll(".image-card")).toHaveLength(30);
    expect(wrapper.text()).toContain("已显示全部图片");

    wrapper.unmount();
  });

  it("首屏未撑满视口时自动继续加载下一页", async () => {
    const page0 = makeImages(24);
    const page1 = makeImages(6).map((image, index) => ({ ...image, id: `img-fill-${index}` }));

    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 30;
      if (cmd === "get_images" && pageOf(args) === 0) return page0;
      if (cmd === "get_images" && pageOf(args) === 1) return page1;
      return [];
    });

    Object.defineProperty(HTMLElement.prototype, "clientHeight", {
      configurable: true,
      get() {
        return this.classList?.contains("gallery-scroll") ? 900 : 0;
      },
    });
    Object.defineProperty(HTMLElement.prototype, "scrollHeight", {
      configurable: true,
      get() {
        return this.classList?.contains("gallery-scroll") ? 640 : 0;
      },
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 1 });
    expect(wrapper.findAll(".image-card")).toHaveLength(30);

    wrapper.unmount();
  });

  it("图库为空时显示图库管理空状态", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });

    const wrapper = mount(LibraryView);
    await flushPromises();

    expect(wrapper.text()).toContain("图库还没有图片");
    expect(wrapper.text()).toContain("这里用于查看全部图片、补充导入、处理导入失败和失效文件。");
    expect(wrapper.text()).toContain("导入后，全部图片会按入库时间显示在这里");
    expect(wrapper.find("[data-section='library-empty-state']").exists()).toBe(true);
    expect(wrapper.find(".toolbar").exists()).toBe(false);
    expect(wrapper.find("[data-action='add-images']").exists()).toBe(false);
    expect(wrapper.find("[data-action='add-folder']").exists()).toBe(false);
    expect(wrapper.get("[data-action='empty-add-images']").text()).toContain("导入图片");
    expect(wrapper.get("[data-action='empty-add-folder']").text()).toContain("导入文件夹");
  });

  it("初始加载失败时显示重试按钮并可重新加载", async () => {
    let firstLoad = true;
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") {
        if (firstLoad) {
          firstLoad = false;
          throw new Error("boom");
        }
        return 2;
      }
      if (cmd === "get_images" && pageOf(args) === 0) return makeImages(2);
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("加载失败，请重试");

    await wrapper.get("[data-action='retry-load']").trigger("click");
    await flushPromises();

    expect(wrapper.findAll(".image-card")).toHaveLength(2);
    wrapper.unmount();
  });

  it("滚动超过一页后显示回到顶部按钮，并平滑滚动到顶部", async () => {
    const images = makeImages(24);
    const scrollToMock = vi.fn();
    HTMLElement.prototype.scrollTo = scrollToMock;

    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 48;
      if (cmd === "get_images" && pageOf(args) === 0) return images;
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
