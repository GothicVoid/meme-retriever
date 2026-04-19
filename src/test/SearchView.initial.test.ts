import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import SearchView from "@/views/SearchView.vue";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const mockInvoke = vi.mocked(invoke);

const mockHomeState = {
  imageCount: 3,
  recentSearches: [{ query: "阿布 撇嘴", updatedAt: 3 }],
  recentUsed: [
    { id: "recent-1", filePath: "/library/recent-1.jpg", fileName: "recent-1.jpg", thumbnailPath: "/library/recent-1_t.jpg", fileFormat: "jpg", fileStatus: "normal", width: 100, height: 100, fileSize: 1, addedAt: 4, useCount: 1, tags: [] },
  ],
  frequentUsed: [
    { id: "home-1", filePath: "/library/home-1.jpg", fileName: "home-1.jpg", thumbnailPath: "/library/home-1_t.jpg", fileFormat: "jpg", fileStatus: "normal", width: 100, height: 100, fileSize: 1, addedAt: 1, useCount: 3, tags: [] },
    { id: "home-2", filePath: "/library/home-2.jpg", fileName: "home-2.jpg", thumbnailPath: "/library/home-2_t.jpg", fileFormat: "jpg", fileStatus: "normal", width: 100, height: 100, fileSize: 1, addedAt: 2, useCount: 2, tags: [] },
    { id: "home-3", filePath: "/library/home-3.jpg", fileName: "home-3.jpg", thumbnailPath: "/library/home-3_t.jpg", fileFormat: "jpg", fileStatus: "normal", width: 100, height: 100, fileSize: 1, addedAt: 3, useCount: 1, tags: [] },
  ],
};

describe("SearchView 初始加载", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("挂载时优先获取首页启动态数据", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("get_home_state");
    expect(mockInvoke).not.toHaveBeenCalledWith("search", expect.objectContaining({ query: "" }));
  });

  it("空查询首页展示常用图片区", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("聊天旁边快速取图");
    expect(wrapper.text()).toContain("直接挑一张");
    expect(wrapper.text()).toContain("先发出去再说");
    expect(wrapper.text()).not.toContain("最近搜索");
    expect(wrapper.find(".search-dock").exists()).toBe(true);
    expect(wrapper.findAll(".image-card")).toHaveLength(4);
    wrapper.unmount();
  });

  it("清空查询回首页后，只有重新聚焦才展示历史下拉", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([
        { id: "a", filePath: "/a.jpg", thumbnailPath: "/a_t.jpg", fileFormat: "jpg", score: 0.9, tags: [], debugInfo: null },
      ]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    await input.setValue("");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(false);

    await input.trigger("focus");
    await flushPromises();

    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(true);

    wrapper.unmount();
  });

  it("图库为空时显示冷启动引导", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") {
        return Promise.resolve({
          imageCount: 0,
          recentSearches: [],
          recentUsed: [],
          frequentUsed: [],
        });
      }
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("先把表情包放进来");
    expect(wrapper.text()).toContain("导入图片");
    wrapper.unmount();
  });

  it("首页数据加载失败时仍保留搜索启动区", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.reject(new Error("home failed"));
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("聊天旁边快速取图");
    expect(wrapper.text()).not.toContain("先把表情包放进来");
    wrapper.unmount();
  });
});
