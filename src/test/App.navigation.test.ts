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

describe("App 工作台壳层", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [];
      }
      return undefined;
    });
  });

  it("搜索首页默认不渲染展开工作台头部", async () => {
    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.find(".app-shell__expanded-toolbar").exists()).toBe(false);
  });

  it("进入图库页时切换为展开工作台头部", async () => {
    const router = createTestRouter();
    await router.push("/library");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.get(".app-shell__expanded-toolbar").text()).toContain("图库整理");
  });

  it("启动时按当前模式调用窗口布局命令", async () => {
    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("apply_window_layout", {
      mode: "sidebar",
      dockSide: "right",
    });
  });

  it("启动时存在未完成入库任务时显示恢复对话框", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }, { id: 2, filePath: "/tmp/b.jpg" }];
      }
      return undefined;
    });

    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.text()).toContain("上次有 2 张图片还没整理完");
    expect(wrapper.text()).toContain("继续处理");
    expect(wrapper.text()).toContain("放弃并清理");
  });
});
