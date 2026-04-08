import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import GlobalProgressBar from "@/components/GlobalProgressBar.vue";
import { useLibraryStore } from "@/stores/library";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", component: { template: "<div/>" } },
    { path: "/library", component: { template: "<div/>" } },
  ],
});

describe("GlobalProgressBar", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("indexing=false 时不渲染", () => {
    const wrapper = mount(GlobalProgressBar, { global: { plugins: [router] } });
    expect(wrapper.find(".global-progress").exists()).toBe(false);
  });

  it("indexing=true 时渲染进度条", async () => {
    const store = useLibraryStore();
    store.indexing = true;
    store.indexTotal = 10;
    store.indexCurrent = 3;
    const wrapper = mount(GlobalProgressBar, { global: { plugins: [router] } });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".global-progress").exists()).toBe(true);
  });

  it("进度宽度正确", async () => {
    const store = useLibraryStore();
    store.indexing = true;
    store.indexTotal = 10;
    store.indexCurrent = 3;
    const wrapper = mount(GlobalProgressBar, { global: { plugins: [router] } });
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".global-progress-fill").attributes("style")).toContain("30%");
  });

  it("点击跳转 /library", async () => {
    const store = useLibraryStore();
    store.indexing = true;
    store.indexTotal = 10;
    store.indexCurrent = 1;
    const wrapper = mount(GlobalProgressBar, { global: { plugins: [router] } });
    await wrapper.vm.$nextTick();
    await wrapper.find(".global-progress").trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/library");
  });
});
