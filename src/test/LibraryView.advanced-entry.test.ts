import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "@/stores/settings";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === "get_image_count") return 0;
    if (cmd === "get_images") return [];
    return [];
  }),
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

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: { template: "<div>search</div>" } },
      { path: "/library", component: { template: "<div>library</div>" } },
      { path: "/private-role-maintenance", component: { template: "<div>private-role</div>" } },
    ],
  });
}

async function mountLibraryView() {
  const router = createTestRouter();
  await router.push("/library");
  await router.isReady();

  const { default: LibraryView } = await import("@/views/LibraryView.vue");
  const wrapper = mount(LibraryView, {
    global: {
      plugins: [router],
    },
  });
  await flushPromises();
  return { wrapper, router };
}

describe("LibraryView 高级能力入口", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("工具条中展示角色维护入口", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => true,
    }));

    const { wrapper } = await mountLibraryView();

    expect(wrapper.text()).toContain("打开角色维护");
    expect(wrapper.text()).toContain("搜不到角色时，可补几张示例图帮助识别。");
    expect(wrapper.find("[data-section='private-role-library-entry']").exists()).toBe(true);
    expect(wrapper.find("[data-action='open-private-role-library']").exists()).toBe(true);
  });

  it("非开发模式下仍展示角色维护入口", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => false,
    }));

    const { wrapper } = await mountLibraryView();

    expect(wrapper.text()).toContain("打开角色维护");
    expect(wrapper.text()).toContain("搜不到角色时，可补几张示例图帮助识别。");
    expect(wrapper.find("[data-action='open-private-role-library']").exists()).toBe(true);
  });

  it("点击入口后跳转到角色维护页", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => true,
    }));

    const { wrapper, router } = await mountLibraryView();

    await wrapper.get("[data-action='open-private-role-library']").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/private-role-maintenance");
  });

  it("页头提供返回搜索按钮，点击后回到搜索页并切回侧边栏态", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => true,
    }));

    const { wrapper, router } = await mountLibraryView();
    const settingsStore = useSettingsStore();
    settingsStore.currentWindowMode = "expanded";

    await wrapper.get("[data-action='back-to-search']").trigger("click");
    await flushPromises();

    expect(router.currentRoute.value.path).toBe("/");
    expect(settingsStore.currentWindowMode).toBe("sidebar");
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("save_window_preferences", {
      mode: "sidebar",
    });
    expect(vi.mocked(invoke)).toHaveBeenCalledWith("apply_window_layout", {
      mode: "sidebar",
    });
  });
});
