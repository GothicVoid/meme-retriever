import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import LibraryView from "@/views/LibraryView.vue";
import type { ImageMeta } from "@/stores/library";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";

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

const mockImages: ImageMeta[] = [
  {
    id: "img-newest",
    filePath: "/library/images/newest.jpg",
    fileName: "newest.jpg",
    thumbnailPath: "/library/thumbs/newest.jpg",
    fileFormat: "jpg",
    fileStatus: "normal",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 3,
    useCount: 0,
    tags: [],
  },
  {
    id: "img-missing",
    filePath: "/library/images/missing.jpg",
    fileName: "missing.jpg",
    thumbnailPath: "/library/thumbs/missing.jpg",
    fileFormat: "jpg",
    fileStatus: "missing",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 2,
    useCount: 0,
    tags: [],
  },
  {
    id: "img-older",
    filePath: "/library/images/older.jpg",
    fileName: "older.jpg",
    thumbnailPath: "/library/thumbs/older.jpg",
    fileFormat: "jpg",
    fileStatus: "normal",
    width: 800,
    height: 600,
    fileSize: 1,
    addedAt: 1,
    useCount: 0,
    tags: [],
  },
];

describe("LibraryView 管理视图", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("显示图库管理标题和管理视图切换入口", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("图库管理");
    expect(wrapper.find("[data-view='all']").exists()).toBe(true);
    expect(wrapper.find("[data-view='recent']").exists()).toBe(true);
    expect(wrapper.find("[data-view='issues']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("切换到最近新增时按新增顺序展示图片", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-view='recent']").trigger("click");
    await flushPromises();

    const cards = wrapper.findAll(".image-card");
    const images = wrapper.findAll(".image-card img");
    expect(cards).toHaveLength(3);
    expect(images[0].attributes("alt")).toBe("img-newest");
    expect(cards[1].find(".img-missing").exists()).toBe(true);
    expect(images[1].attributes("alt")).toBe("img-older");

    wrapper.unmount();
  });

  it("切换到异常图片时只展示失效图片", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-view='issues']").trigger("click");
    await flushPromises();

    const cards = wrapper.findAll(".image-card");
    expect(cards).toHaveLength(1);
    expect(cards[0].find(".img-missing").exists()).toBe(true);
    expect(wrapper.findAll(".image-card img")).toHaveLength(0);

    wrapper.unmount();
  });

  it("异常图片为空时显示分组空态", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 2;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) {
        return mockImages.filter((item) => item.fileStatus === "normal");
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-view='issues']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("当前没有异常图片");

    wrapper.unmount();
  });

  it("存在最近一次导入失败时展示结果摘要，并可查看失败明细", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-a",
          totalCount: 3,
          importedCount: 1,
          duplicatedCount: 1,
          failedCount: 1,
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-a2",
          filePath: "/tmp/imports/a2.jpg",
          errorMessage: "图片已损坏",
        }];
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("最近一次导入");
    expect(wrapper.text()).toContain("新增 1");
    expect(wrapper.text()).toContain("已存在 1");
    expect(wrapper.text()).toContain("失败 1");
    expect(wrapper.get("[data-action='show-import-failures']").text()).toContain("查看失败项");

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("a2.jpg");
    expect(wrapper.text()).toContain("图片已损坏");

    wrapper.unmount();
  });

  it("最近一次导入只有新增成功时，主动作切到最近新增视图", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-b",
          totalCount: 2,
          importedCount: 2,
          duplicatedCount: 0,
          failedCount: 0,
        };
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-latest-imported']").trigger("click");
    await flushPromises();

    expect(wrapper.get("[data-view='recent']").classes()).toContain("active");

    wrapper.unmount();
  });

  it("存在未完成任务时，优先显示恢复横幅而不是最近一次导入结果", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") {
        return [{ id: "task-1", filePath: "/tmp/a.jpg" }];
      }
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-c",
          totalCount: 3,
          importedCount: 1,
          duplicatedCount: 1,
          failedCount: 1,
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-c2",
          filePath: "/tmp/imports/c2.jpg",
          errorMessage: "图片已损坏",
        }];
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("上次有 1 张图片还没整理完");
    expect(wrapper.find('[data-section="latest-import-summary"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("恢复完成后优先展示本次恢复结果，而不是整批历史导入汇总", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-history",
          totalCount: 14,
          importedCount: 11,
          duplicatedCount: 1,
          failedCount: 2,
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-history-2",
          filePath: "/tmp/imports/history-2.jpg",
          errorMessage: "历史失败项",
        }];
      }
      return [];
    });

    const recoveryStore = useTaskRecoveryStore();
    recoveryStore.completedRecoverySummary = {
      totalCount: 3,
      importedCount: 2,
      duplicatedCount: 0,
      failedCount: 1,
      failures: [{
        taskId: "task-recovery-3",
        fileName: "recovery-3.jpg",
        errorMessage: "恢复后仍失败",
      }],
    };

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("刚刚继续处理");
    expect(wrapper.text()).toContain("刚处理完剩余 3 张");
    expect(wrapper.text()).toContain("新增 2");
    expect(wrapper.text()).toContain("失败 1");
    expect(wrapper.text()).not.toContain("共处理 14 张");

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("recovery-3.jpg");
    expect(wrapper.text()).toContain("恢复后仍失败");

    wrapper.unmount();
  });
});
