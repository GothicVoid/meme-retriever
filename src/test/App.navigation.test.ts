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

  it("搜索首页默认不渲染顶部壳层导航", async () => {
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

  it("进入图库页时也不渲染顶部壳层导航", async () => {
    const router = createTestRouter();
    await router.push("/library");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    expect(wrapper.find(".app-shell__expanded-toolbar").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("设置");
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
    });
  });

  it("启动时存在 3 条及以上未完成入库任务时显示恢复对话框", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [
          { id: 1, filePath: "/tmp/a.jpg" },
          { id: 2, filePath: "/tmp/b.jpg" },
          { id: 3, filePath: "/tmp/c.jpg" },
        ];
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

    expect(wrapper.text()).toContain("上次导入中断，还有 3 张图片未处理");
    expect(wrapper.text()).toContain("继续导入");
    expect(wrapper.text()).toContain("放弃剩余图片");
  });
});
