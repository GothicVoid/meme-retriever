import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
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

class MockIntersectionObserver {
  static instances: MockIntersectionObserver[] = [];

  callback: IntersectionObserverCallback;
  observe = vi.fn();
  disconnect = vi.fn();
  unobserve = vi.fn();

  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
    MockIntersectionObserver.instances.push(this);
  }

  trigger(isIntersecting = true) {
    this.callback([{ isIntersecting } as IntersectionObserverEntry], this as unknown as IntersectionObserver);
  }
}

function triggerLoadMore() {
  const observer = MockIntersectionObserver.instances.at(-1);
  expect(observer).toBeDefined();
  observer?.trigger(true);
}

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

function makeResultsWithScores(scores: number[]): SearchResult[] {
  return scores.map((score, i) => ({
    id: `img-${i}`,
    filePath: `/img-${i}.jpg`,
    thumbnailPath: `/thumb-${i}.jpg`,
    fileFormat: "jpg",
    score,
    tags: [],
    debugInfo: null,
  }));
}

describe("SearchView — 结果展示体验", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
    MockIntersectionObserver.instances = [];
    vi.stubGlobal("IntersectionObserver", MockIntersectionObserver);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("高相关结果不足一屏时直接展示全部", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "黄金船";
    store.results = makeResults(5);
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(5);
    expect(wrapper.find("[data-testid='load-more-trigger']").exists()).toBe(false);
  });

  it("高相关结果很多时滚动会继续补齐下一段", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "黄金船";
    store.results = makeResultsWithScores([1, 0.99, 0.98, 0.97, 0.96, 0.95, 0.94, 0.93, 0.92, 0.91, 0.9, 0.89, 0.88]);
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(12);
    expect(wrapper.find("[data-testid='load-more-trigger']").exists()).toBe(true);

    triggerLoadMore();
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(13);
  });

  it("当前已加载结果全是高相关时会继续拉取更多", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    mockInvoke.mockClear();
    const store = useSearchStore();
    store.query = "黄金船";
    store.results = makeResults(30);
    await wrapper.vm.$nextTick();

    triggerLoadMore();
    await wrapper.vm.$nextTick();
    triggerLoadMore();
    await wrapper.vm.$nextTick();
    triggerLoadMore();
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("search", { query: "黄金船", limit: 60 });
  });

  it("高相关和较相关结果会一起展示，只隐藏低相关尾部", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "黄金船";
    store.results = makeResultsWithScores([1, 0.97, 0.95, 0.92, 0.9, 0.88, 0.78, 0.74, 0.7, 0.5]);
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(9);
    expect(wrapper.text()).toContain("找到 9 张和“黄金船”相关的图");
    expect(wrapper.text()).toContain("其余结果先收起来了");
    expect(wrapper.text()).toContain("最像的 9 张已经排在前面");
    expect(wrapper.text()).toContain("查看更多 1 张");
    expect(wrapper.find("[data-action='show-more-secondary']").exists()).toBe(true);
  });

  it("列表分层与统一阈值保持一致", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "黄金船";
    store.results = makeResultsWithScores([0.8, 0.75, 0.7, 0.55, 0.54]);
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(4);
    expect(wrapper.text()).toContain("其余结果先收起来了");
    expect(wrapper.text()).toContain("最像的 4 张已经排在前面");
    expect(wrapper.text()).toContain("查看更多 1 张");
  });

  it("用户主动展开后才显示低相关结果，并可收起", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "黄金船";
    store.results = makeResultsWithScores([1, 0.97, 0.95, 0.92, 0.9, 0.88, 0.78, 0.74, 0.7, 0.5]);
    await wrapper.vm.$nextTick();

    await wrapper.find("[data-action='show-more-secondary']").trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll(".image-card").length).toBe(10);

    await wrapper.find("[data-action='show-less']").trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll(".image-card").length).toBe(9);
  });

  it("整批结果都低相关时优先显示改写建议，不直接灌出一堆图", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "不知道";
    store.results = makeResultsWithScores([0.44, 0.42, 0.4, 0.39]);
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(0);
    expect(wrapper.text()).toContain("没找到足够相关的结果");
    expect(wrapper.text()).toContain("先试试图里的原文");
    expect(wrapper.findAll('[data-testid="search-guidance-item"]').length).toBeGreaterThanOrEqual(3);
  });

  it("整批结果都低相关时用户手动展开会显示全部候选", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "不知道";
    store.results = makeResultsWithScores([0.44, 0.42, 0.4, 0.39]);
    await wrapper.vm.$nextTick();

    await wrapper.find("[data-action='show-more-secondary']").trigger("click");
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".image-card").length).toBe(4);
    expect(wrapper.find("[data-action='show-less']").exists()).toBe(true);
  });

  it("无结果时展示失败反馈区和多条下一步建议", async () => {
    const wrapper = mount(SearchView);
    await flushPromises();
    const store = useSearchStore();
    store.query = "完全搜不到";
    store.results = [];
    await wrapper.vm.$nextTick();

    expect(wrapper.find(".result-feedback").exists()).toBe(true);
    expect(wrapper.text()).toContain("换个说法再试试");
    expect(wrapper.findAll('[data-testid="search-guidance-item"]').length).toBeGreaterThanOrEqual(3);
    expect(wrapper.text()).toContain("试试图片里的原文");
    expect(wrapper.text()).toContain("试试角色名 + 动作");
  });
});
