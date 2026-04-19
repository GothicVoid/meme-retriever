import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import SearchView from "@/views/SearchView.vue";
import Toast from "@/components/Toast.vue";
import { useSearchStore } from "@/stores/search";
import type { ImageMeta } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: vi.fn(),
}));
const copyImageMock = vi.fn();
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: copyImageMock }),
}));

const mockInvoke = vi.mocked(invoke);
const mockConfirm = vi.mocked(confirm);

const mockImage: ImageMeta = {
  id: "uuid-1",
  filePath: "/img.jpg",
  fileName: "img.jpg",
  thumbnailPath: "/thumb.jpg",
  width: 100,
  height: 100,
  addedAt: 0,
  useCount: 0,
  tags: [],
};

const mockHomeState = {
  imageCount: 1,
  recentSearches: [{ query: "阿布 撇嘴", updatedAt: 2 }],
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
  frequentUsed: [{
    id: "home-1",
    filePath: "/img.jpg",
    fileName: "img.jpg",
    thumbnailPath: "/thumb.jpg",
    fileFormat: "jpg",
    fileStatus: "normal",
    width: 100,
    height: 100,
    fileSize: 100,
    addedAt: 0,
    useCount: 2,
    tags: [],
  }],
};

describe("SearchView", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockConfirm.mockReset();
    copyImageMock.mockReset();
  });

  it("首页搜索框显示更明确的占位文案", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();
    expect(wrapper.find("input").attributes("placeholder")).toBe("搜台词、角色、动作、场景");
  });

  it("首页首屏挂载高权重搜索英雄区和示例词样式类", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();

    expect(wrapper.get(".search-view__hero").exists()).toBe(true);
    expect(wrapper.getComponent({ name: "SearchBar" }).classes()).toContain("search-view__search");
    expect(wrapper.getComponent({ name: "SearchBar" }).classes()).toContain("search-view__search--hero");
    expect(wrapper.get(".home-landing__examples").classes()).toContain("search-view__examples");
    expect(wrapper.get(".home-landing__example").classes()).toContain("ui-chip-button");
  });

  it("输入非空查询后切换到搜索结果态", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "阿布" }));
    expect(wrapper.find("input").attributes("placeholder")).toBe("搜台词、角色、动作、场景");
    expect(wrapper.text()).not.toContain("常用表情");
    expect(wrapper.findAll(".image-card")).toHaveLength(2);
  });

  it("拼音组合输入期间不会触发搜索，结束后才搜索", async () => {
    vi.useFakeTimers();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();
    mockInvoke.mockClear();

    const input = wrapper.find("input");
    await input.trigger("compositionstart");
    await input.setValue("a");
    await vi.advanceTimersByTimeAsync(350);
    await flushPromises();
    expect(mockInvoke).not.toHaveBeenCalledWith("search", expect.anything());

    await input.setValue("阿");
    await input.trigger("compositionend");
    await vi.advanceTimersByTimeAsync(350);
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "阿" }));

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("清空查询后回到首页启动态", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") {
        return Promise.resolve(mockResults());
      }
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
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

    expect(wrapper.text()).toContain("常用表情");
    expect(wrapper.text()).toContain("按图片里的字、角色名、动作、场景来找表情");
    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(false);
  });

  it("首页示例词可直接触发搜索，且搜索后不显示历史下拉", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    await wrapper.find(".home-landing__example").trigger("click");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "撤回消息" }));
    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(false);
    expect(wrapper.text()).not.toContain("常用表情");

    wrapper.unmount();
  });

  it("最近搜索存在时展示并支持点击重搜", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();

    expect(wrapper.text()).toContain("最近搜索");
    const button = wrapper.find('[data-testid="recent-search-item"]');
    expect(button.exists()).toBe(true);

    await button.trigger("click");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.find("input").element.value).toBe("阿布 撇嘴");
    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "阿布 撇嘴" }));
  });

  it("搜索框聚焦且输入为空时展示最近搜索下拉", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.trigger("focus");
    await flushPromises();

    const dropdownItems = wrapper.findAll('[data-testid="search-history-dropdown-item"]');
    expect(wrapper.get('[data-testid="search-history-dropdown"]').classes()).toContain("ui-floating-panel");
    expect(dropdownItems).toHaveLength(1);
    expect(dropdownItems[0].text()).toContain("阿布 撇嘴");

    wrapper.unmount();
  });

  it("搜索框有值时不展示最近搜索下拉", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await input.trigger("focus");
    await flushPromises();

    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("按 Esc 会收起最近搜索下拉并保留当前查询", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") {
        return Promise.resolve({
          ...mockHomeState,
          recentSearches: [
            { query: "阿布 撇嘴", updatedAt: 2 },
            { query: "猫猫 心虚", updatedAt: 1 },
          ],
        });
      }
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.trigger("focus");
    await flushPromises();
    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(true);

    await input.setValue("阿布");
    await flushPromises();
    await input.setValue("");
    await flushPromises();
    await input.trigger("focus");
    await flushPromises();
    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(true);

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await flushPromises();

    expect(wrapper.find('[data-testid="search-history-dropdown"]').exists()).toBe(false);
    expect(wrapper.find("input").element.value).toBe("");

    wrapper.unmount();
  });

  it("删除单条最近搜索后刷新首页和下拉", async () => {
    let currentHomeState = {
      ...mockHomeState,
      recentSearches: [
        { query: "阿布 撇嘴", updatedAt: 2 },
        { query: "猫猫 心虚", updatedAt: 1 },
      ],
    };

    mockInvoke.mockImplementation((cmd: string, payload?: Record<string, unknown>) => {
      if (cmd === "get_home_state") return Promise.resolve(currentHomeState);
      if (cmd === "delete_search_history") {
        currentHomeState = {
          ...currentHomeState,
          recentSearches: currentHomeState.recentSearches.filter((item) => item.query !== payload?.query),
        };
        return Promise.resolve(null);
      }
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    await wrapper.find("input").trigger("focus");
    await flushPromises();
    expect(wrapper.findAll('[data-testid="search-history-dropdown-item"]')).toHaveLength(2);

    await wrapper.find('[data-testid="search-history-delete"]').trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("delete_search_history", { query: "阿布 撇嘴" });
    const remainingDropdownItems = wrapper.findAll('[data-testid="search-history-dropdown-item"]');
    const remainingHomeItems = wrapper.findAll('[data-testid="recent-search-item"]');
    expect(remainingDropdownItems).toHaveLength(1);
    expect(remainingDropdownItems[0].text()).toContain("猫猫 心虚");
    expect(remainingHomeItems).toHaveLength(1);
    expect(remainingHomeItems[0].text()).toContain("猫猫 心虚");

    wrapper.unmount();
  });

  it("搜索结果右键删除会调用删除逻辑并从结果中移除", async () => {
    mockConfirm.mockResolvedValue(true);
    mockInvoke.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      if (cmd === "delete_image") {
        expect(args).toEqual({ id: "a" });
        return Promise.resolve(undefined);
      }
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const firstCard = wrapper.find(".image-card-shell");
    await firstCard.trigger("contextmenu", { clientX: 24, clientY: 24 });
    await flushPromises();

    const deleteButton = firstCard.get("[data-action='delete']");
    await deleteButton.trigger("click");
    await flushPromises();

    expect(mockConfirm).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("delete_image", { id: "a" });
    expect(wrapper.findAll(".image-card")).toHaveLength(1);

    wrapper.unmount();
  });

  it("首页卡片右键查看详情会打开详情弹层", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "get_image_meta") {
        return Promise.resolve({
          id: "recent-1",
          filePath: "/recent.jpg",
          fileName: "recent.jpg",
          thumbnailPath: "/recent_t.jpg",
          fileFormat: "jpg",
          width: 100,
          height: 100,
          fileSize: 100,
          fileStatus: "normal",
          addedAt: 10,
          useCount: 1,
          tags: [],
        });
      }
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const firstCard = wrapper.find(".image-card-shell");
    await firstCard.trigger("contextmenu", { clientX: 24, clientY: 24 });
    await flushPromises();

    const openButton = firstCard.findAll(".context-menu button")
      .find((button) => button.text().includes("查看详情"));
    expect(openButton).toBeTruthy();
    await openButton!.trigger("click");
    await flushPromises();

    expect(wrapper.find(".modal-backdrop").exists()).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith("get_image_meta", { id: "recent-1" });

    wrapper.unmount();
  });

  it("搜索结果支持键盘焦点移动，并用 Enter 触发复制", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    expect(wrapper.findAll(".image-card--focused")).toHaveLength(1);

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await flushPromises();
    expect(copyImageMock).toHaveBeenCalledWith("a");

    wrapper.unmount();
  });

  it("搜索结果支持用 Space 打开轻量预览，并用 Esc 关闭", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();

    expect(wrapper.find(".quick-preview-backdrop").exists()).toBe(true);
    expect(wrapper.text()).toContain("查看详情");

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await flushPromises();
    expect(wrapper.find(".quick-preview-backdrop").exists()).toBe(false);

    wrapper.unmount();
  });

  it("按 / 可聚焦搜索框并选中当前输入", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布 撇嘴");
    await flushPromises();
    await input.trigger("blur");
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "/" }));
    await flushPromises();

    const element = input.element as HTMLInputElement;
    expect(document.activeElement).toBe(element);
    expect(element.selectionStart).toBe(0);
    expect(element.selectionEnd).toBe("阿布 撇嘴".length);

    wrapper.unmount();
  });

  it("按 Ctrl+K 可聚焦搜索框并选中当前输入", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("撤回消息");
    await flushPromises();
    await input.trigger("blur");
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }));
    await flushPromises();

    const element = input.element as HTMLInputElement;
    expect(document.activeElement).toBe(element);
    expect(element.selectionStart).toBe(0);
    expect(element.selectionEnd).toBe("撤回消息".length);

    wrapper.unmount();
  });

  it("搜索结果存在时显示结果区快捷键提示", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const hint = wrapper.find('[data-testid="result-shortcuts-hint"]');
    expect(hint.exists()).toBe(true);
    expect(hint.text()).toContain("Enter");
    expect(hint.text()).toContain("复制");
    expect(hint.text()).toContain("Space");
    expect(hint.text()).toContain("预览");

    wrapper.unmount();
  });

  it("搜索过程中显示明确的搜索中提示", async () => {
    let resolveSearch: ((value: ReturnType<typeof mockResults>) => void) | null = null;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") {
        return new Promise((resolve) => {
          resolveSearch = resolve as typeof resolveSearch;
        });
      }
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.text()).toContain("正在搜索相关图片");

    resolveSearch?.(mockResults());
    await flushPromises();

    wrapper.unmount();
  });

  it("搜索失败时展示明确反馈，并保留去图库管理入口", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.reject(new Error("boom"));
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.text()).toContain("这次搜索没成功");
    expect(wrapper.text()).toContain("可以重试，或先去图库管理检查图片状态");

    const galleryAction = wrapper.find('[data-action="go-gallery-management"]');
    expect(galleryAction.exists()).toBe(true);

    await galleryAction.trigger("click");
    expect(window.location.pathname).toBe("/library");
    expect(window.location.search).toContain("view=issues");

    wrapper.unmount();
  });

  it("正式快速预览打开后显示预览快捷键提示", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();

    const hint = wrapper.find('[data-testid="quick-preview-shortcuts-hint"]');
    expect(hint.exists()).toBe(true);
    expect(hint.text()).toContain("Enter");
    expect(hint.text()).toContain("复制");
    expect(hint.text()).toContain("Esc");
    expect(hint.text()).toContain("关闭");

    wrapper.unmount();
  });

  it("搜索结果悬浮轻预览后可通过放大查看进入正式预览，点击卡片仍复制", async () => {
    vi.useFakeTimers();
    copyImageMock.mockResolvedValue(undefined);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await vi.advanceTimersByTimeAsync(350);
    await flushPromises();

    const firstCardShell = wrapper.find(".image-card-shell");
    await firstCardShell.trigger("mouseenter");
    await vi.advanceTimersByTimeAsync(180);
    await flushPromises();

    expect(wrapper.find('[data-testid="hover-preview"]').exists()).toBe(true);

    await wrapper.get('[data-testid="hover-preview-open"]').trigger("click");
    await flushPromises();

    expect(wrapper.find(".quick-preview-backdrop").exists()).toBe(true);

    await wrapper.get(".quick-preview__close").trigger("click");
    await flushPromises();

    await wrapper.find(".image-card").trigger("click");
    await flushPromises();

    expect(copyImageMock).toHaveBeenCalledWith("a");

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("首页卡片悬浮轻预览后可通过放大查看进入正式预览", async () => {
    vi.useFakeTimers();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const firstCardShell = wrapper.find(".image-card-shell");
    await firstCardShell.trigger("mouseenter");
    await vi.advanceTimersByTimeAsync(180);
    await flushPromises();

    expect(wrapper.find('[data-testid="hover-preview"]').exists()).toBe(true);

    await wrapper.get('[data-testid="hover-preview-open"]').trigger("click");
    await flushPromises();

    expect(wrapper.find(".quick-preview-backdrop").exists()).toBe(true);

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("正式快速预览支持在搜索结果中切换，并在关闭后恢复当前图片焦点", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();

    expect(wrapper.find(".quick-preview__image").attributes("src")).toContain("/a.jpg");

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();

    expect(wrapper.find(".quick-preview__image").attributes("src")).toContain("/b.jpg");

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await flushPromises();

    expect(wrapper.find(".quick-preview-backdrop").exists()).toBe(false);
    const focusedCards = wrapper.findAll(".image-card--focused");
    expect(focusedCards).toHaveLength(1);
    expect(focusedCards[0].attributes("alt") ?? focusedCards[0].text()).toBeDefined();

    wrapper.unmount();
  });

  it("正式快速预览支持在文件夹中显示", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();

    await wrapper.get('[data-testid="quick-preview-reveal"]').trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("reveal_in_finder", { id: "a" });

    wrapper.unmount();
  });

  it("正式快速预览打开后收起结果区快捷键提示，只保留预览快捷键提示", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.find('[data-testid="result-shortcuts-hint"]').exists()).toBe(true);

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();

    expect(wrapper.find('[data-testid="result-shortcuts-hint"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="quick-preview-shortcuts-hint"]').exists()).toBe(true);

    wrapper.unmount();
  });

  it("正式快速预览打开缺失文件时显示统一缺图态，并隐藏定位操作", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") {
        return Promise.resolve([{
          id: "missing-1",
          filePath: "/missing.jpg",
          thumbnailPath: "/missing_t.jpg",
          fileFormat: "jpg",
          fileStatus: "missing",
          score: 0.9,
          tags: [],
          debugInfo: null,
        }]);
      }
      return Promise.resolve([]);
    });

    const wrapper = mount({
      components: { SearchView, Toast },
      template: "<div><SearchView /><Toast /></div>",
    }, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("缺图");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();

    expect(wrapper.text()).toContain("原文件已丢失");
    expect(wrapper.text()).toContain("可查看详情重新定位，或删除这条记录。");
    expect(wrapper.find(".quick-preview__image").exists()).toBe(false);
    expect(wrapper.find('[data-testid="quick-preview-reveal"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("正式快速预览内按 Enter 复制当前图片", async () => {
    copyImageMock.mockResolvedValue(undefined);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("阿布");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await flushPromises();

    expect(copyImageMock).toHaveBeenCalledWith("b");

    wrapper.unmount();
  });

  it("首页正式快速预览沿用首页来源列表顺序切换", async () => {
    vi.useFakeTimers();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const firstCardShell = wrapper.find(".image-card-shell");
    await firstCardShell.trigger("mouseenter");
    await vi.advanceTimersByTimeAsync(180);
    await flushPromises();
    await wrapper.get('[data-testid="hover-preview-open"]').trigger("click");
    await flushPromises();

    expect(wrapper.find(".quick-preview__image").attributes("src")).toContain("/recent.jpg");

    await wrapper.get('[data-testid="quick-preview-next"]').trigger("click");
    await flushPromises();

    expect(wrapper.find(".quick-preview__image").attributes("src")).toContain("/img.jpg");

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("最近搜索为空时不显示最近搜索区", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") {
        return Promise.resolve({
          ...mockHomeState,
          recentSearches: [],
        });
      }
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();

    expect(wrapper.text()).not.toContain("最近搜索");
  });

  it("最近用过显示在常用表情之前", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();

    const text = wrapper.text();
    expect(text.indexOf("最近用过")).toBeGreaterThan(-1);
    expect(text.indexOf("常用表情")).toBeGreaterThan(-1);
    expect(text.indexOf("最近用过")).toBeLessThan(text.indexOf("常用表情"));
  });

  it("首页点击图片复制后会刷新首页数据", async () => {
    copyImageMock.mockResolvedValue(undefined);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") {
        return Promise.resolve({
          imageCount: 1,
          recentSearches: [],
          recentUsed: [],
          frequentUsed: [{
            id: "home-1",
            filePath: "/img.jpg",
            fileName: "img.jpg",
            thumbnailPath: "/thumb.jpg",
            fileFormat: "jpg",
            fileStatus: "normal",
            width: 100,
            height: 100,
            fileSize: 100,
            addedAt: 0,
            useCount: 2,
            tags: [],
          }],
        });
      }
      if (cmd === "get_images") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount({
      components: { SearchView, Toast },
      template: "<div><SearchView /><Toast /></div>",
    }, { attachTo: document.body });
    await flushPromises();

    await wrapper.find(".image-card").trigger("click");
    await flushPromises();

    expect(copyImageMock).toHaveBeenCalledWith("home-1");
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "get_home_state")).toHaveLength(2);
  });

  it("点击搜索结果后显示已复制提示", async () => {
    copyImageMock.mockResolvedValue(undefined);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount({
      components: { SearchView, Toast },
      template: "<div><SearchView /><Toast /></div>",
    }, { attachTo: document.body });

    await flushPromises();
    const searchStore = useSearchStore();
    searchStore.query = "阿布";
    searchStore.results = [{
      id: "uuid-1",
      filePath: "/img.jpg",
      thumbnailPath: "/thumb.jpg",
      fileFormat: "jpg",
      score: 1,
      tags: [],
      debugInfo: null,
    }];
    await wrapper.vm.$nextTick();
    await wrapper.find(".image-card").trigger("click");
    await flushPromises();

    const toast = document.body.querySelector(".toast");
    expect(copyImageMock).toHaveBeenCalledWith("uuid-1");
    expect(toast?.textContent).toContain("已复制");

    wrapper.unmount();
  });

  it("开启开发调试模式时显示顶部提示", async () => {
    localStorage.setItem("settings", JSON.stringify({ devDebugMode: true }));
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_images") return Promise.resolve([mockImage]);
      if (cmd === "search") {
        return Promise.resolve([{
          id: "uuid-1",
          filePath: "/img.jpg",
          thumbnailPath: "/thumb.jpg",
          fileFormat: "jpg",
          score: 1,
          tags: [],
          debugInfo: {
            mainRoute: "semantic",
            mainScore: 0.7,
            auxScore: 0.2,
            semScore: 0.9,
            kwScore: 0,
            tagScore: 0,
            popularityBoost: 0.02,
          },
        }]);
      }
      return Promise.resolve(undefined);
    });
    const wrapper = mount(SearchView);
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("调试");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(wrapper.text()).toContain("开发调试模式");
  });

  it("详情页删除事件会调用 delete_image 并关闭弹窗", async () => {
    mockConfirm.mockResolvedValue(true);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_images") return Promise.resolve([mockImage]);
      if (cmd === "get_image_meta") {
        return Promise.resolve({
          ...mockImage,
          fileFormat: "jpg",
          fileStatus: "missing",
          fileSize: 100,
        });
      }
      if (cmd === "delete_image") return Promise.resolve(undefined);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const searchStore = useSearchStore();
    searchStore.query = "删除";
    searchStore.results = [{
      id: "uuid-1",
      filePath: "/img.jpg",
      thumbnailPath: "/thumb.jpg",
      fileFormat: "jpg",
      fileStatus: "missing",
      score: 1,
      tags: [],
      debugInfo: null,
    }];
    await wrapper.vm.$nextTick();
    await wrapper.find(".image-card").trigger("dblclick");
    await flushPromises();

    await wrapper.find(".delete-btn").trigger("click");
    await flushPromises();

    expect(mockConfirm).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("delete_image", { id: "uuid-1" });
    expect(searchStore.results).toEqual([]);
    expect(wrapper.findComponent({ name: "DetailModal" }).exists()).toBe(false);

    wrapper.unmount();
  });

  it("删除后清空查询会重新获取首页数据", async () => {
    mockConfirm.mockResolvedValue(true);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "get_image_meta") {
        return Promise.resolve({
          ...mockImage,
          fileFormat: "jpg",
          fileStatus: "missing",
          fileSize: 100,
        });
      }
      if (cmd === "delete_image") return Promise.resolve(undefined);
      if (cmd === "search") return Promise.resolve(mockResults());
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("删除");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const searchStore = useSearchStore();
    searchStore.results = [{
      id: "uuid-1",
      filePath: "/img.jpg",
      thumbnailPath: "/thumb.jpg",
      fileFormat: "jpg",
      fileStatus: "missing",
      score: 1,
      tags: [],
      debugInfo: null,
    }];
    await wrapper.vm.$nextTick();
    await wrapper.find(".image-card").trigger("dblclick");
    await flushPromises();

    await wrapper.find(".delete-btn").trigger("click");
    await flushPromises();

    await input.setValue("");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "get_home_state")).toHaveLength(2);
    expect(wrapper.text()).toContain("常用表情");
    wrapper.unmount();
  });

  it("搜索失败且存在最近用过时展示快捷入口，并可回到首页启动态", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_home_state") return Promise.resolve(mockHomeState);
      if (cmd === "get_images") return Promise.resolve([]);
      if (cmd === "search") return Promise.resolve([]);
      return Promise.resolve([]);
    });

    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    const input = wrapper.find("input");
    await input.setValue("完全搜不到");
    await flushPromises();
    await new Promise((resolve) => setTimeout(resolve, 350));
    await flushPromises();

    const recentUsedAction = wrapper.find('[data-action="show-recent-used"]');
    expect(recentUsedAction.exists()).toBe(true);

    await recentUsedAction.trigger("click");
    await flushPromises();

    expect(wrapper.find("input").element.value).toBe("");
    expect(wrapper.text()).toContain("最近用过");
    expect(wrapper.text()).toContain("常用表情");

    wrapper.unmount();
  });
});

function mockResults() {
  return [
    { id: "a", filePath: "/a.jpg", thumbnailPath: "/a_t.jpg", fileFormat: "jpg", score: 0.9, tags: [], debugInfo: null },
    { id: "b", filePath: "/b.jpg", thumbnailPath: "/b_t.jpg", fileFormat: "jpg", score: 0.82, tags: [], debugInfo: null },
  ];
}
