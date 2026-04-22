import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import App from "@/App.vue";
import SearchView from "@/views/SearchView.vue";
import LibraryView from "@/views/LibraryView.vue";
import { useLibraryStore } from "@/stores/library";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
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

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: SearchView },
      { path: "/library", component: LibraryView },
    ],
  });
}

function makeHomeState(pendingTaskCount: number) {
  return {
    imageCount: 1,
    pendingTaskCount,
    recentSearches: [],
    recentUsed: [{
      id: "recent-1",
      filePath: "/recent.jpg",
      fileName: "recent.jpg",
      thumbnailPath: "/recent_t.jpg",
      fileFormat: "jpg",
      fileStatus: "normal",
      width: 100,
      height: 100,
      fileSize: 100,
      addedAt: 10,
      useCount: 1,
      tags: [],
    }],
    frequentUsed: [],
  };
}

describe("App 恢复任务状态同步", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
  });

  it("继续导入后首页不再显示恢复角标，而是切换为进行中状态", async () => {
    let pendingCount = 3;
    const eventHandlers: Array<(event: { payload: { status: string } }) => unknown> = [];

    mockListen.mockImplementation(async (_event, handler) => {
      eventHandlers.push(handler as (event: { payload: { status: string } }) => unknown);
      return () => {};
    });

    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return makeHomeState(pendingCount);
      if (cmd === "get_pending_tasks") {
        return Array.from({ length: pendingCount }, (_, index) => ({
          id: `task-${index + 1}`,
          filePath: `/tmp/${index + 1}.jpg`,
          status: index === 0 ? "processing" : "pending",
        }));
      }
      if (cmd === "resume_pending_tasks") return pendingCount;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get('[data-testid="gallery-pending-badge"]').text()).toContain("3");

    await wrapper.get('[data-action="open-gallery-management"]').trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='resume-pending-tasks']").trigger("click");
    await flushPromises();

    expect(wrapper.find(".resume-backdrop").exists()).toBe(false);
    expect(wrapper.find('[data-testid="gallery-pending-badge"]').exists()).toBe(false);

    pendingCount = 2;
    for (const handler of eventHandlers) {
      await handler({ payload: { status: "completed" } });
    }
    await flushPromises();

    expect(wrapper.find('[data-testid="gallery-pending-badge"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("恢复进行中进入图库页会显示进度状态而不是旧的恢复横幅", async () => {
    let pendingCount = 2;
    const eventHandlers: Array<(event: { payload: { status: string } }) => unknown> = [];

    mockListen.mockImplementation(async (_event, handler) => {
      eventHandlers.push(handler as (event: { payload: { status: string } }) => unknown);
      return () => {};
    });

    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return makeHomeState(pendingCount);
      if (cmd === "get_pending_tasks") {
        return Array.from({ length: pendingCount }, (_, index) => ({
          id: `task-${index + 1}`,
          filePath: `/tmp/${index + 1}.jpg`,
          status: index === 0 ? "processing" : "pending",
        }));
      }
      if (cmd === "resume_pending_tasks") return pendingCount;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    await wrapper.get('[data-action="open-gallery-management"]').trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='resume-pending-tasks']").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(wrapper.find(".main-task-card--recovery").exists()).toBe(false);
    expect(wrapper.find(".main-task-card--progress").exists()).toBe(true);
    expect(wrapper.text()).toContain("0/2");

    pendingCount = 1;
    for (const handler of eventHandlers) {
      await handler({ payload: { status: "completed" } });
    }
    await flushPromises();

    expect(wrapper.text()).toContain("1/2");

    wrapper.unmount();
  });

  it("当前会话正在正常导入时不显示恢复对话框", async () => {
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return makeHomeState(2);
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/1.jpg", status: "processing" },
          { id: "task-2", filePath: "/tmp/2.jpg", status: "pending" },
        ];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const libraryStore = useLibraryStore();
    libraryStore.indexing = true;
    await flushPromises();

    expect(wrapper.find(".resume-backdrop").exists()).toBe(false);

    wrapper.unmount();
  });

  it("待恢复数量小于 3 时不弹启动恢复对话框，达到 3 才弹", async () => {
    mockListen.mockResolvedValue(() => {});
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return makeHomeState(2);
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/1.jpg", status: "processing" },
          { id: "task-2", filePath: "/tmp/2.jpg", status: "pending" },
        ];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.find(".resume-backdrop").exists()).toBe(false);
    expect(wrapper.get('[data-testid="gallery-pending-badge"]').text()).toContain("2");

    wrapper.unmount();

    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return makeHomeState(3);
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/1.jpg", status: "processing" },
          { id: "task-2", filePath: "/tmp/2.jpg", status: "pending" },
          { id: "task-3", filePath: "/tmp/3.jpg", status: "pending" },
        ];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const wrapper2 = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper2.find(".resume-backdrop").exists()).toBe(true);

    wrapper2.unmount();
  });

  it("恢复全部完成后进入图库页会优先显示本次恢复结果", async () => {
    let pendingCount = 2;
    const eventHandlers: Array<(event: { payload: { id?: string; status: string; resultKind?: string; file_name?: string; message?: string } }) => unknown> = [];

    mockListen.mockImplementation(async (_event, handler) => {
      eventHandlers.push(handler as (event: { payload: { id?: string; status: string; resultKind?: string; file_name?: string; message?: string } }) => unknown);
      return () => {};
    });

    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return makeHomeState(pendingCount);
      if (cmd === "get_pending_tasks") {
        return Array.from({ length: pendingCount }, (_, index) => ({
          id: `task-${index + 1}`,
          filePath: `/tmp/${index + 1}.jpg`,
          status: index === 0 ? "processing" : "pending",
        }));
      }
      if (cmd === "resume_pending_tasks") return pendingCount;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-history",
          totalCount: 14,
          importedCount: 11,
          duplicatedCount: 1,
          failedCount: 2,
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "history-fail-1",
          filePath: "/tmp/history-fail-1.jpg",
          errorMessage: "历史失败项",
        }];
      }
      return [];
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    await wrapper.get('[data-action="open-gallery-management"]').trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='resume-pending-tasks']").trigger("click");
    await flushPromises();

    pendingCount = 1;
    for (const handler of eventHandlers) {
      await handler({
        payload: {
          id: "task-1",
          status: "completed",
          resultKind: "imported",
          file_name: "first.jpg",
        },
      });
    }
    await flushPromises();

    pendingCount = 0;
    for (const handler of eventHandlers) {
      await handler({
        payload: {
          id: "task-2",
          status: "error",
          resultKind: "failed",
          file_name: "second.jpg",
          message: "恢复后仍失败",
        },
      });
    }
    await flushPromises();

    await router.push("/library");
    await flushPromises();

    expect(wrapper.text()).toContain("刚刚继续导入");
    expect(wrapper.text()).toContain("刚导完剩余 2 张");
    expect(wrapper.text()).toContain("新增 1");
    expect(wrapper.text()).toContain("失败 1");
    expect(wrapper.text()).not.toContain("共处理 14 张");

    wrapper.unmount();
  });
});
