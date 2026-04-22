import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import LibraryView from "@/views/LibraryView.vue";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  confirm: vi.fn(),
}));

vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const mockInvoke = vi.mocked(invoke);

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/library", component: LibraryView },
    ],
  });
}

describe("LibraryView 路由视图恢复", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_image_count") return 0;
      if (cmd === "get_images") return [];
      return [];
    });
  });

  it("view=issues 时忽略旧视图参数并保持全部图片视图", async () => {
    const router = createTestRouter();
    await router.push("/library?view=issues");
    await router.isReady();

    const wrapper = mount(LibraryView, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get("[data-view='all']").classes()).toContain("active");
    expect(wrapper.find("[data-view='issues']").exists()).toBe(false);
  });

  it("view=recent 时忽略旧视图参数并保持全部图片视图", async () => {
    const router = createTestRouter();
    await router.push("/library?view=recent");
    await router.isReady();

    const wrapper = mount(LibraryView, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get("[data-view='all']").classes()).toContain("active");
    expect(wrapper.find("[data-view='recent']").exists()).toBe(false);
  });
});
