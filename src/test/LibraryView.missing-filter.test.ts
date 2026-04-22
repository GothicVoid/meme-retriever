import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
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
const mockConfirm = vi.mocked(confirm);

const mixedImages: ImageMeta[] = [
  {
    id: "img-normal",
    filePath: "/library/images/normal.jpg",
    fileName: "normal.jpg",
    thumbnailPath: "/library/thumbs/normal.jpg",
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
    id: "img-missing-a",
    filePath: "/library/images/missing-a.jpg",
    fileName: "missing-a.jpg",
    thumbnailPath: "/library/thumbs/missing-a.jpg",
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
    id: "img-missing-b",
    filePath: "/library/images/missing-b.jpg",
    fileName: "missing-b.jpg",
    thumbnailPath: "/library/thumbs/missing-b.jpg",
    fileFormat: "jpg",
    fileStatus: "missing",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 1,
    useCount: 0,
    tags: [],
  },
];

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: "/library", component: LibraryView }],
  });
}

describe("LibraryView 失效图片过滤态", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockConfirm.mockReset();
    HTMLElement.prototype.scrollTo = vi.fn();
  });

  it("存在失效图片时显示查看失效图片入口，不存在时不显示", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mixedImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find("[data-action='view-missing-images']").exists()).toBe(true);
    wrapper.unmount();

    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 1;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) {
        return mixedImages.filter((image) => image.fileStatus !== "missing");
      }
      return [];
    });

    const wrapperWithoutMissing = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapperWithoutMissing.find("[data-action='view-missing-images']").exists()).toBe(false);
    wrapperWithoutMissing.unmount();
  });

  it("点击查看失效图片后进入同页过滤态，只展示 missing 项", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mixedImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-missing-images']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("正在查看已发现的失效图片，共 2 张");
    expect(wrapper.find("[data-action='view-all-images']").exists()).toBe(true);
    expect(wrapper.findAll(".image-card")).toHaveLength(2);
    expect(wrapper.findAll(".img-missing")).toHaveLength(2);
    expect(wrapper.findAll(".image-card img")).toHaveLength(0);
    expect(wrapper.find("[data-action='clear-missing']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("点击查看全部图片后恢复完整列表", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mixedImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-missing-images']").trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='view-all-images']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).not.toContain("正在查看已发现的失效图片");
    expect(wrapper.findAll(".image-card")).toHaveLength(3);
    expect(wrapper.find("[data-action='clear-missing']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("fileStatus=missing 时直接进入失效图片过滤态", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mixedImages;
      return [];
    });

    const router = createTestRouter();
    await router.push("/library?fileStatus=missing");
    await router.isReady();

    const wrapper = mount(LibraryView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.text()).toContain("正在查看已发现的失效图片，共 2 张");
    expect(wrapper.findAll(".image-card")).toHaveLength(2);

    wrapper.unmount();
  });

  it("过滤态下确认后调用 clear_missing_images 并退出过滤态", async () => {
    let cleared = false;
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return cleared ? 1 : 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) {
        return cleared ? [mixedImages[0]] : mixedImages;
      }
      if (cmd === "clear_missing_images") {
        cleared = true;
        return 2;
      }
      return [];
    });
    mockConfirm.mockResolvedValueOnce(true);

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-missing-images']").trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='clear-missing']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("clear_missing_images");
    expect(wrapper.find("[data-action='view-all-images']").exists()).toBe(false);
    expect(wrapper.findAll(".image-card")).toHaveLength(1);

    wrapper.unmount();
  });
});
