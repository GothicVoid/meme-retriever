import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Event } from "@tauri-apps/api/event";
import type { ImageMeta } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
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

const mockImage: ImageMeta = {
  id: "uuid-1",
  filePath: "/img.jpg",
  fileName: "img.jpg",
  thumbnailPath: "/thumb.jpg",
  fileFormat: "jpg",
  width: 100,
  height: 100,
  addedAt: 0,
  useCount: 0,
  tags: [],
};

const mockHomeState = {
  imageCount: 1,
  recentSearches: [],
  recentUsed: [],
  frequentUsed: [],
};

async function loadSearchView(devMode: boolean) {
  vi.resetModules();
  vi.doMock("@/utils/runtime", () => ({
    isDevelopmentMode: () => devMode,
  }));

  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const { confirm } = await import("@tauri-apps/plugin-dialog");
  const { default: SearchView } = await import("@/views/SearchView.vue");

  return {
    SearchView,
    mockInvoke: vi.mocked(invoke),
    mockListen: vi.mocked(listen),
    mockConfirm: vi.mocked(confirm),
  };
}

describe("SearchView 开发工具浮层", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    vi.doUnmock("@/utils/runtime");
  });

  it("开发模式下显示扳手按钮和开发工具浮层", async () => {
    const { SearchView, mockInvoke, mockListen } = await loadSearchView(true);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    mockListen.mockResolvedValue(() => {});

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find("[data-action='toggle-dev-tools']").exists()).toBe(true);

    await wrapper.get("[data-action='toggle-dev-tools']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("开发工具");
    expect(wrapper.text()).toContain("开发调试模式");
    expect(wrapper.text()).toContain("重新生成图像索引");
    expect(wrapper.text()).toContain("清空图库");

    wrapper.unmount();
  });

  it("非开发模式下不显示扳手按钮", async () => {
    const { SearchView, mockInvoke, mockListen } = await loadSearchView(false);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    mockListen.mockResolvedValue(() => {});

    const wrapper = mount(SearchView);
    await flushPromises();

    expect(wrapper.find("[data-action='toggle-dev-tools']").exists()).toBe(false);
  });

  it("点击重新生成图像索引会调用命令并显示完成态", async () => {
    let reindexHandler!: (event: Event<unknown>) => void;
    const { SearchView, mockInvoke, mockListen } = await loadSearchView(true);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "reindex_all") return Promise.resolve(undefined);
      return Promise.resolve([]);
    });
    mockListen.mockImplementation(async (eventName, handler) => {
      if (eventName === "reindex-progress") {
        reindexHandler = handler as (event: Event<unknown>) => void;
      }
      return () => {};
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='toggle-dev-tools']").trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='reindex-all']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("reindex_all");

    reindexHandler({ payload: { current: 3, total: 3 } } as Event<unknown>);
    await flushPromises();

    expect(wrapper.text()).toContain("索引重建完成");
    expect(wrapper.find(".search-view__dev-tools-progress-bar").exists()).toBe(false);

    wrapper.unmount();
  });

  it("点击清空图库会先确认再调用 clear_gallery", async () => {
    vi.useFakeTimers();

    let clearHandler!: (event: Event<unknown>) => void;
    let resolveClear!: (value: unknown) => void;
    const { SearchView, mockInvoke, mockListen, mockConfirm } = await loadSearchView(true);
    mockConfirm.mockResolvedValue(true);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([mockImage]);
      if (cmd === "clear_gallery") {
        return new Promise((resolve) => {
          resolveClear = resolve;
        });
      }
      return Promise.resolve([]);
    });
    mockListen.mockImplementation(async (eventName, handler) => {
      if (eventName === "clear-gallery-progress") {
        clearHandler = handler as (event: Event<unknown>) => void;
      }
      return () => {};
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='toggle-dev-tools']").trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='clear-gallery']").trigger("click");
    await flushPromises();

    expect(mockConfirm).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("clear_gallery");

    clearHandler({ payload: { current: 1, total: 1 } } as Event<unknown>);
    resolveClear(undefined);
    await vi.runAllTimersAsync();
    await flushPromises();
    vi.useRealTimers();

    wrapper.unmount();
  });
});
