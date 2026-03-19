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

const mockResults = [
  { id: "new", filePath: "/library/new.jpg", thumbnailPath: "/library/new_t.jpg", score: 1.0, tags: [] },
  { id: "mid", filePath: "/library/mid.jpg", thumbnailPath: "/library/mid_t.jpg", score: 1.0, tags: [] },
  { id: "old", filePath: "/library/old.jpg", thumbnailPath: "/library/old_t.jpg", score: 1.0, tags: [] },
];

describe("SearchView 初始加载", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("挂载时自动触发空查询搜索", async () => {
    mockInvoke.mockResolvedValueOnce(mockResults);
    mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "" }));
  });

  it("空查询返回结果时显示图片列表", async () => {
    mockInvoke.mockResolvedValueOnce(mockResults);
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.findAll(".image-card")).toHaveLength(3);
    wrapper.unmount();
  });

  it("空查询无结果时显示提示文案", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const wrapper = mount(SearchView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("没找到");
    wrapper.unmount();
  });
});
