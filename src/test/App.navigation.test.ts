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

  it("应用外壳挂载统一主题入口和页面骨架类", async () => {
    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const shell = wrapper.get('[data-ui-theme="memedesk"]');
    expect(shell.classes()).toContain("app-shell");
    expect(wrapper.get("nav").classes()).toContain("app-nav");
    expect(wrapper.get("main").classes()).toContain("app-shell__content");
  });

  it("一级导航链接挂载统一基础导航类", async () => {
    const router = createTestRouter();
    await router.push("/");
    await router.isReady();

    const wrapper = mount(App, {
      global: {
        plugins: [router],
      },
    });
    await flushPromises();

    const links = wrapper.findAll(".app-nav__link");
    expect(links).toHaveLength(3);
  });

  it("启动时存在未完成入库任务时显示恢复对话框", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }, { id: 2, filePath: "/tmp/b.jpg" }];
      }
      return [];
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

  it("点击继续处理会调用恢复任务命令", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }];
      }
      if (cmd === "resume_pending_tasks") {
        return 1;
      }
      return [];
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

    await wrapper.get("[data-action='resume-pending-tasks']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("resume_pending_tasks");
    expect(wrapper.text()).not.toContain("上次有 1 张图片还没整理完");
  });

  it("点击放弃并清理会调用清空任务队列命令", async () => {
    mockInvoke.mockImplementation(async (cmd) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: 1, filePath: "/tmp/a.jpg" }];
      }
      if (cmd === "clear_task_queue") {
        return undefined;
      }
      return [];
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

    await wrapper.get("[data-action='clear-pending-tasks']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("clear_task_queue");
    expect(wrapper.text()).not.toContain("上次有 1 张图片还没整理完");
  });
});
