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

describe("SearchView", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockConfirm.mockReset();
    copyImageMock.mockReset();
  });

  it("图库为空时显示引导文案", async () => {
    mockInvoke.mockResolvedValue([]);
    const wrapper = mount(SearchView);
    await flushPromises();
    expect(wrapper.text()).toContain("还没有图片哦");
  });

  it("有图片但搜索无结果时显示搜索提示", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_images") return Promise.resolve([mockImage]);
      return Promise.resolve([]);
    });
    const wrapper = mount(SearchView);
    await flushPromises();
    expect(wrapper.text()).toContain("没找到相关图片");
  });

  it("点击搜索结果后显示已复制提示", async () => {
    copyImageMock.mockResolvedValue(undefined);
    mockInvoke.mockResolvedValue([]);

    const wrapper = mount({
      components: { SearchView, Toast },
      template: "<div><SearchView /><Toast /></div>",
    }, { attachTo: document.body });

    await flushPromises();
    const searchStore = useSearchStore();
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
});
