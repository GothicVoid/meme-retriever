import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import LibraryView from "@/views/LibraryView.vue";

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

import { confirm } from "@tauri-apps/plugin-dialog";

const mockInvoke = vi.mocked(invoke);
const mockConfirm = vi.mocked(confirm);

const mockImages = [
  {
    id: "uuid-1",
    filePath: "/library/images/uuid-1.jpg",
    fileName: "sample.jpg",
    thumbnailPath: "/library/thumbs/uuid-1.jpg",
    width: 800,
    height: 600,
    addedAt: 1700000000,
    useCount: 0,
    tags: [],
  },
  {
    id: "uuid-2",
    filePath: "/library/images/uuid-2.jpg",
    fileName: "sample2.jpg",
    thumbnailPath: "/library/thumbs/uuid-2.jpg",
    width: 400,
    height: 400,
    addedAt: 1700000001,
    useCount: 0,
    tags: [],
  },
];

describe("LibraryView 删除功能", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockConfirm.mockReset();
  });

  it("图库加载后显示图片列表", async () => {
    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.findAll(".image-card")).toHaveLength(2);
    wrapper.unmount();
  });

  it("ImageCard 触发 delete 事件后弹出确认对话框", async () => {
    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockConfirm.mockResolvedValueOnce(false); // 用户取消

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    // 触发第一张图片的 delete 事件
    const card = wrapper.findAllComponents({ name: "ImageCard" })[0];
    await card.trigger("contextmenu");
    const deleteButton = document.body.querySelector("[data-action='delete']") as HTMLButtonElement;
    deleteButton.click();
    await flushPromises();

    expect(mockConfirm).toHaveBeenCalledWith(
      expect.stringContaining("删除"),
      expect.objectContaining({ title: expect.any(String) })
    );
    wrapper.unmount();
  });

  it("确认删除后调用 delete_image 并从列表移除", async () => {
    mockInvoke.mockResolvedValueOnce(2); // get_image_count
    mockInvoke.mockResolvedValueOnce(mockImages); // get_images
    mockConfirm.mockResolvedValueOnce(true); // 用户确认
    mockInvoke.mockResolvedValueOnce(undefined); // delete_image

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    const card = wrapper.findAllComponents({ name: "ImageCard" })[0];
    await card.trigger("contextmenu");
    const deleteButton = document.body.querySelector("[data-action='delete']") as HTMLButtonElement;
    deleteButton.click();
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("delete_image", { id: "uuid-1" });
    expect(wrapper.findAll(".image-card")).toHaveLength(1);
    wrapper.unmount();
  });

  it("取消确认不删除图片", async () => {
    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockConfirm.mockResolvedValueOnce(false); // 用户取消

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    const card = wrapper.findAllComponents({ name: "ImageCard" })[0];
    await card.trigger("contextmenu");
    const deleteButton = document.body.querySelector("[data-action='delete']") as HTMLButtonElement;
    deleteButton.click();
    await flushPromises();

    expect(mockInvoke).not.toHaveBeenCalledWith("delete_image", expect.anything());
    expect(wrapper.findAll(".image-card")).toHaveLength(2);
    wrapper.unmount();
  });
});
