import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import SearchView from "@/views/SearchView.vue";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: vi.fn(),
}));
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const mockHomeState = {
  imageCount: 1,
  pendingTaskCount: 0,
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

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: SearchView },
      { path: "/library", component: { template: "<div>library</div>" } },
      { path: "/private-role-maintenance", component: { template: "<div>role-lib</div>" } },
    ],
  });
}

describe("SearchView 搜索失败修复闭环", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockListen.mockResolvedValue(() => {});
  });

  it("无结果时默认推荐查看最近新增，并跳转到最近新增视图", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("完全搜不到");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const action = wrapper.find('[data-action="primary-recovery-action"]');
    expect(action.exists()).toBe(true);
    expect(action.text()).toContain("查看最近新增");

    await action.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.view).toBe("recent");

    wrapper.unmount();
  });

  it("低相关结果时默认推荐查看失效图片，并跳转到失效图片过滤态", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") {
        return Promise.resolve([
          { id: "low-1", filePath: "/low-1.jpg", thumbnailPath: "/low-1_t.jpg", fileFormat: "jpg", score: 0.2, tags: [], debugInfo: null },
          { id: "low-2", filePath: "/low-2.jpg", thumbnailPath: "/low-2_t.jpg", fileFormat: "jpg", score: 0.1, tags: [], debugInfo: null },
        ]);
      }
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("有点像但不准");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const action = wrapper.find('[data-action="primary-recovery-action"]');
    expect(action.exists()).toBe(true);
    expect(action.text()).toContain("查看失效图片");

    await action.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.fileStatus).toBe("missing");

    wrapper.unmount();
  });

  it("存在未完成任务时，搜索失败默认推荐去图库继续导入", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") {
        return Promise.resolve({
          ...mockHomeState,
          pendingTaskCount: 2,
        });
      }
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.reject(new Error("boom"));
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const action = wrapper.find('[data-action="primary-recovery-action"]');
    expect(action.exists()).toBe(true);
    expect(action.text()).toContain("去图库继续导入");

    await action.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.view).toBe("recent");

    wrapper.unmount();
  });

  it("角色名搜不到时默认推荐维护角色示例图", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const action = wrapper.find('[data-action="primary-recovery-action"]');
    expect(action.exists()).toBe(true);
    expect(action.text()).toContain("维护角色示例图");

    await action.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/private-role-maintenance");

    wrapper.unmount();
  });

  it("角色名搜不到时不会误导去图库治理", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const action = wrapper.find('[data-action="primary-recovery-action"]');
    expect(action.exists()).toBe(true);
    expect(action.text()).toContain("维护角色示例图");
    expect(wrapper.text()).not.toContain("查看最近新增");
    expect(wrapper.text()).not.toContain("查看异常图片");

    wrapper.unmount();
  });

  it("导入处理中搜索时显示低干扰的不完整结果提示，完成后消失", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("处理中");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.text()).not.toContain("当前结果可能不完整");

    const libraryStoreModule = await import("@/stores/library");
    const libraryStore = libraryStoreModule.useLibraryStore();
    libraryStore.indexing = true;
    libraryStore.indexCurrent = 1;
    libraryStore.indexTotal = 4;
    await flushPromises();

    expect(wrapper.text()).toContain("当前结果可能不完整");

    libraryStore.indexing = false;
    await flushPromises();

    expect(wrapper.text()).not.toContain("当前结果可能不完整");

    wrapper.unmount();
  });

  it("恢复进行中时首页图库入口显示处理中承接文案，不再显示待恢复角标", async () => {
    let progressHandler: ((event: { payload: { id?: string; status: string; resultKind?: string } }) => void) | null = null;
    mockListen.mockImplementation(async (_event, handler) => {
      progressHandler = handler as typeof progressHandler;
      return () => {};
    });

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") {
        return Promise.resolve({
          ...mockHomeState,
          pendingTaskCount: 2,
        });
      }
      if (cmd === "get_pending_tasks") {
        return Promise.resolve([
          { id: "task-1", filePath: "/tmp/a.jpg", status: "processing" },
          { id: "task-2", filePath: "/tmp/b.jpg", status: "pending" },
        ]);
      }
      if (cmd === "resume_pending_tasks") return Promise.resolve(2);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(SearchView, {
      attachTo: document.body,
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get('[data-testid="gallery-pending-badge"]').text()).toContain("2");

    const recoveryStoreModule = await import("@/stores/taskRecovery");
    const recoveryStore = recoveryStoreModule.useTaskRecoveryStore();
    await recoveryStore.resumePendingTasks();
    await flushPromises();

    expect(wrapper.find('[data-testid="gallery-pending-badge"]').exists()).toBe(false);
    expect(wrapper.get('[data-action="open-gallery-management"]').text()).toContain("正在继续导入");

    progressHandler?.({ payload: { id: "task-1", status: "completed", resultKind: "imported" } });
    await flushPromises();

    expect(wrapper.get('[data-action="open-gallery-management"]').text()).toContain("1/2");

    wrapper.unmount();
  });
});
