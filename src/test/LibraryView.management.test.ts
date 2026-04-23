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

const mockImagesWithoutMissing = mockImages.map((image) => ({
  ...image,
  fileStatus: "normal" as const,
}));

describe("LibraryView 管理视图", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    HTMLElement.prototype.scrollTo = vi.fn();
    localStorage.clear();
  });

  it("显示图库管理标题、主视图入口和新的导入工具条文案", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("图库管理");
    expect(wrapper.text()).not.toContain("在这里导入、整理和排查图片问题");
    expect(wrapper.find("[data-view='all']").exists()).toBe(true);
    expect(wrapper.find("[data-view='recent']").exists()).toBe(false);
    expect(wrapper.find("[data-view='issues']").exists()).toBe(false);
    expect(wrapper.get("[data-action='add-images']").text()).toContain("导入图片");
    expect(wrapper.get("[data-action='add-folder']").text()).toContain("导入文件夹");

    wrapper.unmount();
  });

  it("默认在全部图片视图展示图库列表", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    const cards = wrapper.findAll(".image-card");
    const images = wrapper.findAll(".image-card img");
    expect(cards).toHaveLength(3);
    expect(images[0].attributes("alt")).toBe("img-newest");
    expect(cards[1].find(".img-missing").exists()).toBe(true);
    expect(images[1].attributes("alt")).toBe("img-missing");
    expect(images[2].attributes("alt")).toBe("img-older");

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
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-a2",
          filePath: "/tmp/imports/a2.jpg",
          errorMessage: "图片已损坏",
          failureKind: "file_damaged",
          retryable: false,
          userMessage: "图片文件可能已损坏，暂时无法导入。",
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
    expect(wrapper.text()).toContain("图片文件可能已损坏，暂时无法导入。");
    expect(wrapper.find("[data-action='show-import-failures']").exists()).toBe(false);
    expect(wrapper.get("[data-action='dismiss-import-summary']").text()).toContain("知道了");

    wrapper.unmount();
  });

  it("存在可重试失败时展开后显示重试导入动作", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-retryable",
          totalCount: 1,
          importedCount: 0,
          duplicatedCount: 0,
          failedCount: 1,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-r1",
          filePath: "/tmp/imports/retry.jpg",
          errorMessage: "import interrupted",
          failureKind: "interrupted_recoverable",
          retryable: true,
          userMessage: "导入被中断了，可以继续导入剩余图片。",
        }];
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("导入被中断了，可以继续导入剩余图片。");
    expect(wrapper.get("[data-action='retry-import-failures']").text()).toContain("重试导入");
    expect(wrapper.find("[data-action='dismiss-import-summary']").exists()).toBe(true);
    expect(wrapper.find("[data-action='show-import-failures']").exists()).toBe(false);

    wrapper.unmount();
  });

  it("普通不可重试失败不显示重试导入，并提供知道了动作", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-non-retry",
          totalCount: 2,
          importedCount: 1,
          duplicatedCount: 0,
          failedCount: 1,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-n1",
          filePath: "/tmp/imports/damaged.jpg",
          errorMessage: "damaged file",
          failureKind: "file_damaged",
          retryable: false,
          userMessage: "图片文件可能已损坏，暂时无法导入。",
        }];
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();

    expect(wrapper.find("[data-action='retry-import-failures']").exists()).toBe(false);
    expect(wrapper.get("[data-action='dismiss-import-summary']").text()).toContain("知道了");
    expect(wrapper.get("[data-action='view-latest-imported']").text()).toContain("查看本次新增");

    wrapper.unmount();
  });

  it("混合失败时展示混合说明，并仍以重试导入作为主动作", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-mixed",
          totalCount: 3,
          importedCount: 1,
          duplicatedCount: 0,
          failedCount: 2,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [
          {
            taskId: "task-m1",
            filePath: "/tmp/imports/retry.jpg",
            errorMessage: "interrupted",
            failureKind: "interrupted_recoverable",
            retryable: true,
            userMessage: "导入被中断了，可以继续导入剩余图片。",
          },
          {
            taskId: "task-m2",
            filePath: "/tmp/imports/missing.jpg",
            errorMessage: "file not found",
            failureKind: "file_missing",
            retryable: false,
            userMessage: "原文件不存在，已跳过这张图片。",
          },
        ];
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();

    expect(wrapper.get("[data-action='retry-import-failures']").exists()).toBe(true);
    expect(wrapper.get("[data-action='dismiss-import-summary']").exists()).toBe(true);

    wrapper.unmount();
  });

  it("最近一次导入点击查看本次新增后停留在全部图片并滚动到顶部", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-b",
          totalCount: 2,
          importedCount: 2,
          duplicatedCount: 0,
          failedCount: 0,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      return [];
    });

    const scrollToMock = vi.fn();
    HTMLElement.prototype.scrollTo = scrollToMock;
    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-latest-imported']").trigger("click");
    await flushPromises();

    expect(wrapper.get("[data-view='all']").classes()).toContain("active");
    expect(wrapper.find("[data-view='recent']").exists()).toBe(false);
    expect(scrollToMock).toHaveBeenCalledWith({ top: 0, behavior: "smooth" });
    expect(wrapper.find('[data-section="latest-import-position-tip"]').exists()).toBe(true);
    expect(wrapper.text()).toContain("已定位到本次新增图片");

    wrapper.unmount();
  });

  it("查看本次新增后仅为本轮新增图片显示新角标", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-new",
          totalCount: 3,
          importedCount: 2,
          duplicatedCount: 0,
          failedCount: 1,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-latest-imported']").trigger("click");
    await flushPromises();

    const newBadges = wrapper.findAll(".status-badge--new");
    expect(newBadges).toHaveLength(2);
    expect(newBadges[0].text()).toContain("新");
    expect(newBadges[1].text()).toContain("新");
    expect(wrapper.findAll(".image-card--focused")).toHaveLength(2);

    wrapper.unmount();
  });

  it("导入结果出现后即展示本轮新增的新角标，但未点击前不显示定位提示", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-auto",
          totalCount: 2,
          importedCount: 2,
          duplicatedCount: 0,
          failedCount: 0,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.findAll(".status-badge--new")).toHaveLength(2);
    expect(wrapper.find('[data-section="latest-import-position-tip"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("超过承接窗口的历史导入摘要不再显示", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-stale",
          totalCount: 2,
          importedCount: 1,
          duplicatedCount: 0,
          failedCount: 1,
          completedAt: Math.floor(Date.now() / 1000) - 31 * 60,
        };
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find('[data-section="latest-import-summary"]').exists()).toBe(false);

    wrapper.unmount();
  });

  it("普通导入摘要点击知道了后会退场，并记住当前批次已处理", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-ack",
          totalCount: 2,
          importedCount: 1,
          duplicatedCount: 0,
          failedCount: 1,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-ack-1",
          filePath: "/tmp/imports/ack.jpg",
          errorMessage: "damaged file",
          failureKind: "file_damaged",
          retryable: false,
          userMessage: "图片文件可能已损坏，暂时无法导入。",
        }];
      }
      return [];
    });

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();
    await wrapper.get("[data-action='dismiss-import-summary']").trigger("click");
    await flushPromises();

    expect(wrapper.find('[data-section="latest-import-summary"]').exists()).toBe(false);
    expect(localStorage.getItem("library.latestImportSummaryAck")).toBe("batch-ack");

    wrapper.unmount();
  });

  it("新的导入结果会覆盖上一轮本次新增标记", async () => {
    const scrollToMock = vi.fn();
    HTMLElement.prototype.scrollTo = scrollToMock;
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const recoveryStore = useTaskRecoveryStore();
    recoveryStore.completedRecoverySummary = {
      totalCount: 2,
      importedCount: 2,
      duplicatedCount: 0,
      failedCount: 0,
      failures: [],
    };

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-latest-imported']").trigger("click");
    await flushPromises();
    expect(wrapper.findAll(".status-badge--new")).toHaveLength(2);

    recoveryStore.completedRecoverySummary = {
      totalCount: 1,
      importedCount: 1,
      duplicatedCount: 0,
      failedCount: 0,
      failures: [],
    };
    await flushPromises();

    expect(wrapper.findAll(".status-badge--new")).toHaveLength(1);
    expect(wrapper.findAll(".image-card--focused")).toHaveLength(1);

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
          completedAt: Math.floor(Date.now() / 1000),
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

    expect(wrapper.text()).toContain("上次导入中断，还有 1 张图片未处理");
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
          completedAt: Math.floor(Date.now() / 1000),
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

    expect(wrapper.text()).toContain("刚刚继续导入");
    expect(wrapper.text()).toContain("刚导完剩余 3 张");
    expect(wrapper.text()).toContain("新增 2");
    expect(wrapper.text()).toContain("失败 1");
    expect(wrapper.text()).not.toContain("共处理 14 张");

    wrapper.unmount();
  });

  it("恢复结果查看失败项后立即退场，并回落到普通导入历史摘要", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImages;
      if (cmd === "get_latest_import_summary") {
        return {
          batchId: "batch-history",
          totalCount: 6,
          importedCount: 4,
          duplicatedCount: 1,
          failedCount: 1,
          completedAt: Math.floor(Date.now() / 1000),
        };
      }
      if (cmd === "get_import_batch_failures") {
        return [{
          taskId: "task-history-6",
          filePath: "/tmp/imports/history-6.jpg",
          errorMessage: "历史失败项",
        }];
      }
      return [];
    });

    const recoveryStore = useTaskRecoveryStore();
    recoveryStore.completedRecoverySummary = {
      totalCount: 2,
      importedCount: 1,
      duplicatedCount: 0,
      failedCount: 1,
      failures: [{
        taskId: "task-recovery-2",
        fileName: "recovery-2.jpg",
        errorMessage: "恢复失败",
      }],
    };

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.text()).toContain("刚刚继续导入");

    await wrapper.get("[data-action='show-import-failures']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("recovery-2.jpg");
    expect(wrapper.find("[data-action='dismiss-import-summary']").exists()).toBe(true);
    expect(wrapper.find("[data-action='show-import-failures']").exists()).toBe(false);

    await wrapper.get("[data-action='dismiss-import-summary']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("最近一次导入");
    expect(wrapper.text()).toContain("共处理 6 张");
    expect(wrapper.text()).not.toContain("刚导完剩余 2 张");

    wrapper.unmount();
  });

  it("恢复结果查看本次新增后保持全部图片视图并立即退场，同时显示定位反馈", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const recoveryStore = useTaskRecoveryStore();
    recoveryStore.completedRecoverySummary = {
      totalCount: 2,
      importedCount: 2,
      duplicatedCount: 0,
      failedCount: 0,
      failures: [],
    };

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    await wrapper.get("[data-action='view-latest-imported']").trigger("click");
    await flushPromises();

    expect(wrapper.get("[data-view='all']").classes()).toContain("active");
    expect(wrapper.find('[data-section="latest-import-summary"]').exists()).toBe(false);
    expect(wrapper.findAll(".status-badge--new")).toHaveLength(2);
    expect(wrapper.find('[data-section="latest-import-position-tip"]').exists()).toBe(true);

    wrapper.unmount();
  });

  it("处理中禁用治理操作并显示原因文案，同时不再提供旧视图切换", async () => {
    mockInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === "get_pending_tasks") return [];
      if (cmd === "get_image_count") return 3;
      if (cmd === "get_images" && (!args || (args as { page?: number }).page === 0)) return mockImagesWithoutMissing;
      if (cmd === "get_latest_import_summary") return null;
      return [];
    });

    const recoveryStore = useTaskRecoveryStore();
    recoveryStore.activeRecovery = true;
    recoveryStore.recoveryTotal = 3;
    recoveryStore.recoveryImported = 1;

    const wrapper = mount(LibraryView, { attachTo: document.body });
    await flushPromises();

    expect(wrapper.find("[data-action='view-missing-images']").exists()).toBe(false);
    expect(wrapper.text()).toContain("导入处理中，完成后再整理图库");
    expect(wrapper.find("[data-view='recent']").exists()).toBe(false);
    expect(wrapper.find("[data-view='issues']").exists()).toBe(false);

    wrapper.unmount();
  });
});
