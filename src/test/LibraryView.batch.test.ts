import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import LibraryView from "@/views/LibraryView.vue";
import { useLibraryStore } from "@/stores/library";
import type { ImageMeta } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
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

const mockImages: ImageMeta[] = [
  { id: "uuid-1", filePath: "/img1.jpg", fileName: "img1.jpg", thumbnailPath: "/t1.jpg", width: 100, height: 100, addedAt: 0, useCount: 0, tags: [] },
  { id: "uuid-2", filePath: "/img2.jpg", fileName: "img2.jpg", thumbnailPath: "/t2.jpg", width: 100, height: 100, addedAt: 0, useCount: 0, tags: [] },
];

function mockGallery(images = mockImages) {
  mockInvoke.mockImplementation(async (cmd, args) => {
    if (cmd === "get_pending_tasks") return [];
    if (cmd === "get_image_count") return images.length;
    if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return images;
    if (cmd === "get_latest_import_summary") return null;
    if (cmd === "delete_images") return null;
    return [];
  });
}

describe("LibraryView 批量删除", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockConfirm.mockReset();
  });

  it("默认不显示删除选中按钮，保留批量删除入口", async () => {
    mockGallery();
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.find("[data-action='delete-selected']").exists()).toBe(false);
    expect(wrapper.find("[data-action='enter-batch-delete']").exists()).toBe(true);
    wrapper.unmount();
  });

  it("点击批量删除后进入选择模式并显示 checkbox", async () => {
    mockGallery();
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='enter-batch-delete']").trigger("click");
    await wrapper.vm.$nextTick();

    expect(wrapper.find("[data-action='delete-selected']").exists()).toBe(true);
    expect(wrapper.find("input[type='checkbox']").exists()).toBe(true);
    wrapper.unmount();
  });

  it("批量删除确认框显示选中数量", async () => {
    mockGallery();
    mockConfirm.mockResolvedValueOnce(false);
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='enter-batch-delete']").trigger("click");
    await wrapper.vm.$nextTick();
    const checkboxes = wrapper.findAll("input[type='checkbox']");
    await checkboxes[0].trigger("change");
    await checkboxes[1].trigger("change");

    await wrapper.find("[data-action='delete-selected']").trigger("click");
    await flushPromises();
    expect(mockConfirm).toHaveBeenCalledWith(
      expect.stringContaining("2"),
      expect.anything()
    );
    wrapper.unmount();
  });

  it("取消选择模式后隐藏 checkbox 并清空选择", async () => {
    mockGallery();
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='enter-batch-delete']").trigger("click");
    await wrapper.find("input[type='checkbox']").trigger("change");
    await wrapper.get("[data-action='cancel-selection']").trigger("click");
    await wrapper.vm.$nextTick();

    const store = useLibraryStore();
    expect(wrapper.find("input[type='checkbox']").exists()).toBe(false);
    expect(store.selectedIds.size).toBe(0);
    wrapper.unmount();
  });

  it("默认不显示清除失效图片按钮", async () => {
    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find("[data-action='clear-missing']").exists()).toBe(false);
    wrapper.unmount();
  });

  it("未进入失效图片过滤态时无法直接清除失效图片", async () => {
    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find("[data-action='clear-missing']").exists()).toBe(false);

    wrapper.unmount();
  });

  it("确认后调用 clear_missing_images 并刷新图库", async () => {
    let cleared = false;
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return cleared ? 1 : 2;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) {
        return cleared
          ? [mockImages[0]]
          : [{ ...mockImages[0], fileStatus: "missing" }, mockImages[1]];
      }
      if (cmd === "clear_missing_images") {
        cleared = true;
        return 1;
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

    expect(mockConfirm).toHaveBeenCalledWith(
      expect.stringContaining("清除所有失效图片"),
      expect.objectContaining({ title: "清除失效图片" })
    );
    expect(mockInvoke).toHaveBeenCalledWith("clear_missing_images");
    expect(mockInvoke).toHaveBeenLastCalledWith("get_images", { page: 0 });

    wrapper.unmount();
  });
});
