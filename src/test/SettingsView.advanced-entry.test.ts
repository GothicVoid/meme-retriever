import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: vi.fn(),
}));

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: { template: "<div>search</div>" } },
      { path: "/settings", component: { template: "<div>settings</div>" } },
      { path: "/private-role-maintenance", component: { template: "<div>private-role</div>" } },
    ],
  });
}

async function mountSettingsView() {
  const router = createTestRouter();
  await router.push("/settings");
  await router.isReady();

  const { default: SettingsView } = await import("@/views/SettingsView.vue");
  return mount(SettingsView, {
    global: {
      plugins: [router],
    },
  });
}

describe("SettingsView 高级入口", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.doUnmock("@/utils/runtime");
  });

  it("设置页按基础设置、高级能力和开发调试分组展示", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => true,
    }));

    const wrapper = await mountSettingsView();

    expect(wrapper.text()).toContain("基础设置");
    expect(wrapper.text()).toContain("高级能力");
    expect(wrapper.text()).toContain("开发调试");
  });

  it("开发模式下展示私有角色库入口和说明文案", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => true,
    }));

    const wrapper = await mountSettingsView();

    expect(wrapper.find("[data-action='open-private-role-library']").exists()).toBe(true);
    expect(wrapper.text()).toContain("私有角色库");
    expect(wrapper.text()).toContain("冷门角色或私有对象");
  });

  it("非开发模式下不展示私有角色库入口", async () => {
    vi.resetModules();
    vi.doMock("@/utils/runtime", () => ({
      isDevelopmentMode: () => false,
    }));

    const wrapper = await mountSettingsView();

    expect(wrapper.find("[data-action='open-private-role-library']").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("私有角色库");
  });
});
