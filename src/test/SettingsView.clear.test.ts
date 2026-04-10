import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import SettingsView from "@/views/SettingsView.vue";
import { useLibraryStore } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: vi.fn(),
}));

import { confirm } from "@tauri-apps/plugin-dialog";

const mockInvoke = vi.mocked(invoke);
const mockConfirm = vi.mocked(confirm);

describe("SettingsView 清空图库", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockConfirm.mockReset();
  });

  it("图库为空时按钮禁用", () => {
    const wrapper = mount(SettingsView);
    const button = wrapper.get("[data-action='clear-gallery']");
    expect(button.attributes("disabled")).toBeDefined();
  });

  it("图库有图片时按钮可用", () => {
    const store = useLibraryStore();
    store.images = [
      {
        id: "uuid-1",
        filePath: "/tmp/a.jpg",
        fileName: "a.jpg",
        thumbnailPath: "/tmp/a-thumb.jpg",
        fileFormat: "jpg",
        width: 100,
        height: 100,
        fileSize: 10,
        addedAt: 1,
        useCount: 0,
        tags: [],
      },
    ];

    const wrapper = mount(SettingsView);
    const button = wrapper.get("[data-action='clear-gallery']");
    expect(button.attributes("disabled")).toBeUndefined();
  });

  it("点击时弹出确认对话框", async () => {
    const store = useLibraryStore();
    store.images = [
      {
        id: "uuid-1",
        filePath: "/tmp/a.jpg",
        fileName: "a.jpg",
        thumbnailPath: "/tmp/a-thumb.jpg",
        fileFormat: "jpg",
        width: 100,
        height: 100,
        fileSize: 10,
        addedAt: 1,
        useCount: 0,
        tags: [],
      },
    ];
    mockConfirm.mockResolvedValueOnce(false);

    const wrapper = mount(SettingsView);
    await wrapper.get("[data-action='clear-gallery']").trigger("click");

    expect(mockConfirm).toHaveBeenCalledWith(
      expect.stringContaining("清空"),
      expect.objectContaining({ title: "清空图库" })
    );
  });

  it("取消确认时不调用 clearGallery", async () => {
    const store = useLibraryStore();
    store.images = [
      {
        id: "uuid-1",
        filePath: "/tmp/a.jpg",
        fileName: "a.jpg",
        thumbnailPath: "/tmp/a-thumb.jpg",
        fileFormat: "jpg",
        width: 100,
        height: 100,
        fileSize: 10,
        addedAt: 1,
        useCount: 0,
        tags: [],
      },
    ];
    const clearSpy = vi.spyOn(store, "clearGallery").mockResolvedValue(undefined);
    mockConfirm.mockResolvedValueOnce(false);

    const wrapper = mount(SettingsView);
    await wrapper.get("[data-action='clear-gallery']").trigger("click");

    expect(clearSpy).not.toHaveBeenCalled();
  });

  it("确认后调用 clearGallery", async () => {
    const store = useLibraryStore();
    store.images = [
      {
        id: "uuid-1",
        filePath: "/tmp/a.jpg",
        fileName: "a.jpg",
        thumbnailPath: "/tmp/a-thumb.jpg",
        fileFormat: "jpg",
        width: 100,
        height: 100,
        fileSize: 10,
        addedAt: 1,
        useCount: 0,
        tags: [],
      },
    ];
    const clearSpy = vi.spyOn(store, "clearGallery").mockResolvedValue(undefined);
    mockConfirm.mockResolvedValueOnce(true);

    const wrapper = mount(SettingsView);
    await wrapper.get("[data-action='clear-gallery']").trigger("click");
    await flushPromises();

    expect(clearSpy).toHaveBeenCalled();
  });

  it("clearing 时按钮禁用", () => {
    const store = useLibraryStore();
    store.images = [
      {
        id: "uuid-1",
        filePath: "/tmp/a.jpg",
        fileName: "a.jpg",
        thumbnailPath: "/tmp/a-thumb.jpg",
        fileFormat: "jpg",
        width: 100,
        height: 100,
        fileSize: 10,
        addedAt: 1,
        useCount: 0,
        tags: [],
      },
    ];
    store.clearing = true;

    const wrapper = mount(SettingsView);
    const button = wrapper.get("[data-action='clear-gallery']");
    expect(button.attributes("disabled")).toBeDefined();
  });
});
