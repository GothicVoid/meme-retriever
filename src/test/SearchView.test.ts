import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import SearchView from "@/views/SearchView.vue";
import Toast from "@/components/Toast.vue";
import { useSearchStore } from "@/stores/search";
import type { ImageMeta } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
const copyImageMock = vi.fn();
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: copyImageMock }),
}));

const mockInvoke = vi.mocked(invoke);

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
});
