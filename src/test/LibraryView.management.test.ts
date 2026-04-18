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

const mockImages: ImageMeta[] = [
  {
    id: "img-newest",
    filePath: "/library/images/newest.jpg",
    fileName: "newest.jpg",
    thumbnailPath: "/library/thumbs/newest.jpg",
    fileFormat: "jpg",
    fileStatus: "normal",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 3,
    useCount: 0,
    tags: [],
  },
  {
    id: "img-missing",
    filePath: "/library/images/missing.jpg",
    fileName: "missing.jpg",
    thumbnailPath: "/library/thumbs/missing.jpg",
    fileFormat: "jpg",
    fileStatus: "missing",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 2,
    useCount: 0,
    tags: [],
  },
  {
    id: "img-older",
    filePath: "/library/images/older.jpg",
    fileName: "older.jpg",
    thumbnailPath: "/library/thumbs/older.jpg",
    fileFormat: "jpg",
    fileStatus: "normal",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 1,
    useCount: 0,
    tags: [],
  },
];

describe("LibraryView 管理视图", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("显示图库管理标题和管理视图切换入口", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("图库管理");
    expect(wrapper.find("[data-view='all']").exists()).toBe(true);
    expect(wrapper.find("[data-view='recent']").exists()).toBe(true);
    expect(wrapper.find("[data-view='issues']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("切换到最近新增时按新增顺序展示图片", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-view='recent']").trigger("click");
    await flushPromises();

    const cards = wrapper.findAll(".image-card");
    const images = wrapper.findAll(".image-card img");
    expect(cards).toHaveLength(3);
    expect(images[0].attributes("alt")).toBe("img-newest");
    expect(cards[1].find(".img-missing").exists()).toBe(true);
    expect(images[1].attributes("alt")).toBe("img-older");

    wrapper.unmount();
  });

  it("切换到异常图片时只展示失效图片", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-view='issues']").trigger("click");
    await flushPromises();

    const cards = wrapper.findAll(".image-card");
    expect(cards).toHaveLength(1);
    expect(cards[0].find(".img-missing").exists()).toBe(true);
    expect(wrapper.findAll(".image-card img")).toHaveLength(0);

    wrapper.unmount();
  });

  it("异常图片为空时显示分组空态", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 2;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) {
        return mockImages.filter((item) => item.fileStatus === "normal");
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-view='issues']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("当前没有异常图片");

    wrapper.unmount();
  });
});
