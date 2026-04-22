import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import LibraryView from "@/views/LibraryView.vue";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => new Promise(() => {})), // 永不 resolve，进度永远不完成
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  confirm: vi.fn(),
}));

vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

describe("LibraryView 进度条", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockOpen.mockReset();
  });

  it("添加图片过程中显示进度条和计数", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_pending_tasks") return Promise.resolve([]);
      if (cmd === "get_image_count") return Promise.resolve(0);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "import_entries") return Promise.resolve(2);
      return Promise.resolve([]);
    });

    mockOpen.mockResolvedValueOnce(["/tmp/a.jpg", "/tmp/b.jpg"]);

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises(); // 完成 get_images

    await wrapper.get("[data-action='add-images']").trigger("click");
    await flushPromises();

    expect(wrapper.find(".main-task-card--progress .progress-bar").exists()).toBe(true);
    expect(wrapper.find(".main-task-card--progress").text()).toContain("0/2");
    expect(mockInvoke).toHaveBeenCalledWith("import_entries", {
      entries: [
        { kind: "file", path: "/tmp/a.jpg" },
        { kind: "file", path: "/tmp/b.jpg" },
      ],
    });

    wrapper.unmount();
  });

  it("添加文件夹过程中显示进度条和计数", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_pending_tasks") return Promise.resolve([]);
      if (cmd === "get_image_count") return Promise.resolve(0);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "import_entries") return Promise.resolve(3);
      return Promise.resolve([]);
    });

    mockOpen.mockResolvedValueOnce("/tmp/memes");

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='add-folder']").trigger("click");
    await flushPromises();

    expect(wrapper.find(".main-task-card--progress .progress-bar").exists()).toBe(true);
    expect(wrapper.find(".main-task-card--progress").text()).toContain("0/3");
    expect(mockInvoke).toHaveBeenCalledWith("import_entries", {
      entries: [{ kind: "directory", path: "/tmp/memes" }],
    });

    wrapper.unmount();
  });

  it("未入库时不显示进度条", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_pending_tasks") return Promise.resolve([]);
      if (cmd === "get_image_count") return Promise.resolve(0);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find(".main-task-card--progress .progress-bar").exists()).toBe(false);
    wrapper.unmount();
  });
});
