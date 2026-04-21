import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";

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

  it("展示角色识别增强入口和说明文案", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => true,
    }));

    const { wrapper } = await mountLibraryView();

    expect(wrapper.text()).toContain("高级能力");
    expect(wrapper.text()).toContain("角色识别增强");
    expect(wrapper.text()).toContain("冷门角色或私有对象");
    expect(wrapper.find("[data-action='open-private-role-library']").exists()).toBe(true);
  });

  it("非开发模式下仍展示角色识别增强入口", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => false,
    }));

    const { wrapper } = await mountLibraryView();

    expect(wrapper.text()).toContain("角色识别增强");
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
});
