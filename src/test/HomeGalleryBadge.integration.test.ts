import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter, RouterView } from "vue-router";
import { defineComponent } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import SearchView from "@/views/SearchView.vue";
import LibraryView from "@/views/LibraryView.vue";
import { useLibraryStore } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
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
const mockListen = vi.mocked(listen);

const homeStateWithPending = {
  imageCount: 1,
  pendingTaskCount: 2,
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

function createIntegrationRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: SearchView },
      { path: "/library", component: LibraryView },
    ],
  });
}

const RouterHarness = defineComponent({
  name: "RouterHarness",
  components: { RouterView },
  template: "<RouterView />",
});

describe("首页图库角标与异常恢复联动", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockListen.mockResolvedValue(() => {});
  });

  it("存在未完成任务时首页图库按钮显示角标，点击后进入图库并看到恢复入口", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return homeStateWithPending;
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/a.jpg", status: "pending" },
          { id: "task-2", filePath: "/tmp/b.jpg", status: "processing" },
        ];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createIntegrationRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(RouterHarness, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get('[data-testid="gallery-pending-badge"]').text()).toContain("2");

    await wrapper.get('[data-action="open-gallery-management"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.view).toBe("recent");
    expect(wrapper.text()).toContain("上次导入中断，还有 2 张图片未处理");
    expect(wrapper.find("[data-action='resume-pending-tasks']").exists()).toBe(true);
    expect(wrapper.find("[data-action='clear-pending-tasks']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("有角标时进入图库后可以继续导入未完成任务", async () => {
    type ProgressHandler = (event: { payload: { id: string; status: string } }) => void;
    let progressHandler: ProgressHandler | null = null;
    mockListen.mockImplementation(async (_event, handler) => {
      progressHandler = handler as ProgressHandler;
      return () => {};
    });

    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return homeStateWithPending;
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/a.jpg", status: "pending" },
          { id: "task-2", filePath: "/tmp/b.jpg", status: "processing" },
        ];
      }
      if (cmd === "resume_pending_tasks") return 2;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createIntegrationRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(RouterHarness, {
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

    expect(mockInvoke).toHaveBeenCalledWith("resume_pending_tasks");
    expect(wrapper.text()).not.toContain("上次导入中断，还有 2 张图片未处理");
    expect(wrapper.find(".main-task-card--progress").exists()).toBe(true);
    expect(wrapper.text()).toContain("0/2");

    expect(progressHandler).not.toBeNull();
    progressHandler!({ payload: { id: "task-1", status: "completed" } });
    await flushPromises();

    expect(wrapper.text()).toContain("1/2");

    wrapper.unmount();
  });

  it("有角标时进入图库后可以放弃剩余图片", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") return homeStateWithPending;
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/a.jpg", status: "pending" },
          { id: "task-2", filePath: "/tmp/b.jpg", status: "processing" },
        ];
      }
      if (cmd === "clear_task_queue") return undefined;
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createIntegrationRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(RouterHarness, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    await wrapper.get('[data-action="open-gallery-management"]').trigger("click");
    await flushPromises();

    await wrapper.get("[data-action='clear-pending-tasks']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("clear_task_queue");
    expect(wrapper.text()).not.toContain("上次导入中断，还有 2 张图片未处理");
    expect(wrapper.find("[data-action='resume-pending-tasks']").exists()).toBe(false);
    expect(wrapper.find("[data-action='clear-pending-tasks']").exists()).toBe(false);

    wrapper.unmount();
  });

  it("首页角标数量与图库内未完成任务数量保持一致", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") {
        return {
          ...homeStateWithPending,
          pendingTaskCount: 3,
        };
      }
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/a.jpg", status: "pending" },
          { id: "task-2", filePath: "/tmp/b.jpg", status: "processing" },
          { id: "task-3", filePath: "/tmp/c.jpg", status: "pending" },
        ];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createIntegrationRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(RouterHarness, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get('[data-testid="gallery-pending-badge"]').text()).toContain("3");

    await wrapper.get('[data-action="open-gallery-management"]').trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.view).toBe("recent");
    expect(wrapper.text()).toContain("上次导入中断，还有 3 张图片未处理");

    wrapper.unmount();
  });

  it("当前会话正在正常导入时首页不显示恢复角标", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_home_state") {
        return {
          ...homeStateWithPending,
          pendingTaskCount: 2,
        };
      }
      if (cmd === "get_pending_tasks") {
        return [
          { id: "task-1", filePath: "/tmp/a.jpg", status: "processing" },
          { id: "task-2", filePath: "/tmp/b.jpg", status: "pending" },
        ];
      }
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const router = createIntegrationRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(RouterHarness, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const libraryStore = useLibraryStore();
    libraryStore.indexing = true;
    await flushPromises();

    expect(wrapper.find('[data-testid="gallery-pending-badge"]').exists()).toBe(false);

    wrapper.unmount();
  });
});
