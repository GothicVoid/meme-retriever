import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

describe("LibraryView 入库恢复提示", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockListen.mockResolvedValue(() => {});
  });

  it("存在未完成任务时展示恢复提示条", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }, { id: 2, filePath: "/tmp/b.jpg" }];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("上次导入中断，还有 2 张图片未处理");
    expect(wrapper.find("[data-action='resume-pending-tasks']").exists()).toBe(true);
    expect(wrapper.find("[data-action='clear-pending-tasks']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("只有 processing 任务时也展示恢复提示条", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg", status: "processing" }];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("上次导入中断，还有 1 张图片未处理");
    expect(wrapper.find("[data-action='resume-pending-tasks']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("点击继续导入后隐藏恢复提示，并接入入库进度状态", async () => {
    let progressHandler: ((event: { payload: { id: string; status: string } }) => void) | null = null;
    mockListen.mockImplementation(async (_event, handler) => {
      progressHandler = handler as typeof progressHandler;
      return () => {};
    });

    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }, { id: 2, filePath: "/tmp/b.jpg" }];
      }
      if (cmd === "resume_pending_tasks") return 2;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='resume-pending-tasks']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("resume_pending_tasks");
    expect(wrapper.text()).not.toContain("上次导入中断，还有 2 张图片未处理");
    expect(wrapper.find(".main-task-card--progress").exists()).toBe(true);
    expect(wrapper.text()).toContain("0/2");

    progressHandler?.({ payload: { id: "a", status: "completed" } });
    await flushPromises();
    expect(wrapper.text()).toContain("1/2");

    wrapper.unmount();
  });

  it("点击放弃剩余图片后隐藏恢复提示", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }];
      }
      if (cmd === "clear_task_queue") return undefined;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='clear-pending-tasks']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("clear_task_queue");
    expect(wrapper.text()).not.toContain("上次导入中断，还有 1 张图片未处理");

    wrapper.unmount();
  });
});
