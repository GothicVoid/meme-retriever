import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import SearchView from "@/views/SearchView.vue";
import { useSearchStore } from "@/stores/search";
import type { SearchResult } from "@/stores/search";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));
vi.mock("@/components/DetailModal.vue", () => ({
  default: {
    name: "DetailModal",
    props: ["imageId", "images"],
    emits: ["close"],
    template: `<div class="detail-modal-stub" />`,
  },
}));

const mockInvoke = vi.mocked(invoke);

function makeResults(count: number): SearchResult[] {
  return Array.from({ length: count }, (_, i) => ({
    id: `img-${i}`,
    filePath: `/img-${i}.jpg`,
    thumbnailPath: `/thumb-${i}.jpg`,
    fileFormat: "jpg",
    score: 1 - i * 0.01,
    tags: [],
    debugInfo: null,
  }));
}

describe("SearchView — 分页", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
  });

  it("结果 ≤9 张时不显示「展示更多」按钮", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.results = makeResults(9);
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-action='show-more']").exists()).toBe(false);
  });

  it("结果 >9 张时显示「展示更多」按钮", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.results = makeResults(15);
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-action='show-more']").exists()).toBe(true);
  });

  it("默认只渲染前 9 张 ImageCard", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.results = makeResults(15);
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll(".image-card").length).toBe(9);
  });

  it("点击「展示更多」后渲染 21 张", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.results = makeResults(21);
    await wrapper.vm.$nextTick();
    await wrapper.find("[data-action='show-more']").trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll(".image-card").length).toBe(21);
  });

  it("点击「展示更多」后按钮消失（结果恰好 21 张）", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.results = makeResults(21);
    await wrapper.vm.$nextTick();
    await wrapper.find("[data-action='show-more']").trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-action='show-more']").exists()).toBe(false);
  });

  it("「展示更多」按钮显示剩余数量", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.results = makeResults(15);
    await wrapper.vm.$nextTick();
    expect(wrapper.find("[data-action='show-more']").text()).toContain("6");
  });
});

describe("SearchView — DetailModal 集成", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
  });

  it("初始不显示 DetailModal", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    expect(wrapper.find(".detail-modal-stub").exists()).toBe(false);
  });
});
