import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
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

const mockHomeState = {
  imageCount: 1,
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
    ],
  });
}

describe("SearchView 搜索失败修复闭环", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("无结果时展示去图库管理入口，并跳转到最近新增视图", async () => {
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

    const action = wrapper.find('[data-action="go-gallery-management"]');
    expect(action.exists()).toBe(true);

    await action.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.view).toBe("recent");

    wrapper.unmount();
  });

  it("低相关结果时展示去图库管理入口，并跳转到异常图片视图", async () => {
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

    const action = wrapper.find('[data-action="go-gallery-management"]');
    expect(action.exists()).toBe(true);

    await action.trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/library");
    expect(router.currentRoute.value.query.view).toBe("issues");

    wrapper.unmount();
  });
});
