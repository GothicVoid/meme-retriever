import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createMemoryHistory } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import App from "@/App.vue";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockInvoke = vi.mocked(invoke);

function createTestRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: { template: "<div>search</div>" } },
      { path: "/library", component: { template: "<div>library</div>" } },
      { path: "/settings", component: { template: "<div>settings</div>" } },
      { path: "/private-role-maintenance", component: { template: "<div>private-role</div>" } },
    ],
  });
}

describe("App 一级导航", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
  });

  it("顶层导航只显示首页搜索、图库管理和设置", async () => {
    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const links = wrapper.findAll(".nav a");
    expect(links).toHaveLength(3);
    expect(links.map((item) => item.text())).toEqual(["首页 / 搜索", "图库管理", "设置"]);
  });

  it("顶层导航不再暴露私有角色库维护入口", async () => {
    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.text()).not.toContain("私有角色库维护");
  });

  it("切换到图库管理页时保持对应导航高亮", async () => {
    const router = createTestRouter();
    await router.push("/library");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const activeLink = wrapper.get(".nav a.router-link-active");
    expect(activeLink.text()).toBe("图库管理");
  });
});
