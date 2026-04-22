import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { Event } from "@tauri-apps/api/event";
import { flushPromises } from "@vue/test-utils";
import { useLibraryStore, type ImageMeta, type ImportEntry } from "@/stores/library";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { listen } from "@tauri-apps/api/event";

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const mockImages: ImageMeta[] = [
  {
    id: "uuid-1",
    filePath: "/library/images/uuid-1.jpg",
    fileName: "sample.jpg",
    thumbnailPath: "/library/thumbs/uuid-1.jpg",
    width: 800,
    height: 600,
    addedAt: 1700000000,
    useCount: 0,
    tags: [],
  },
  {
    id: "uuid-2",
    filePath: "/library/images/uuid-2.jpg",
    fileName: "sample_blank.jpg",
    thumbnailPath: "/library/thumbs/uuid-2.jpg",
    width: 400,
    height: 400,
    addedAt: 1700000001,
    useCount: 0,
    tags: [],
  },
];

describe("useLibraryStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
  });

  it("初始状态正确", () => {
    const store = useLibraryStore();
    expect(store.images).toEqual([]);
    expect(store.loading).toBe(false);
    expect(store.total).toBe(0);
  });

  it("fetchImages 调用 invoke 并更新 images 列表", async () => {
    mockInvoke.mockResolvedValueOnce(mockImages);

    const store = useLibraryStore();
    await store.fetchImages();

    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 0 });
    expect(store.images).toEqual(mockImages);
    expect(store.loading).toBe(false);
  });

  it("fetchImages 在 append=true 时追加图片", async () => {
    mockInvoke.mockResolvedValueOnce([mockImages[0]]);
    mockInvoke.mockResolvedValueOnce([mockImages[1]]);

    const store = useLibraryStore();
    await store.fetchImages(0);
    await store.fetchImages(1, true);

    expect(store.images).toEqual(mockImages);
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "get_images", { page: 1 });
  });

  it("fetchImageCount 调用 get_image_count 并更新 total", async () => {
    mockInvoke.mockResolvedValueOnce(23);

    const store = useLibraryStore();
    await store.fetchImageCount();

    expect(mockInvoke).toHaveBeenCalledWith("get_image_count");
    expect(store.total).toBe(23);
  });

  it("fetchImages 过程中 loading 为 true", async () => {
    let resolve!: (v: unknown) => void;
    mockInvoke.mockReturnValueOnce(new Promise((r) => { resolve = r; }));

    const store = useLibraryStore();
    const p = store.fetchImages();
    expect(store.loading).toBe(true);

    resolve([]);
    await p;
    expect(store.loading).toBe(false);
  });

  it("importEntries 完成后图库列表包含新增图片，thumbnailPath 正确", async () => {
    vi.useFakeTimers();

    // listen 立即触发回调，模拟两张图片入库完成
    mockListen.mockImplementation((_event, handler) => {
      handler({ payload: { id: "uuid-1", status: "completed" } } as Event<unknown>);
      handler({ payload: { id: "uuid-2", status: "completed" } } as Event<unknown>);
      return Promise.resolve(() => {});
    });

    const entries: ImportEntry[] = [
      { kind: "file", path: "/tmp/sample.jpg" },
      { kind: "file", path: "/tmp/sample_blank.jpg" },
    ];
    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockInvoke.mockResolvedValueOnce(mockImages.length);

    const store = useLibraryStore();
    const p = store.importEntries(entries);

    // 推进 polling interval
    await vi.runAllTimersAsync();
    await p;

    vi.useRealTimers();

    expect(store.images).toEqual(mockImages);
    expect(store.images[0].thumbnailPath).toBe("/library/thumbs/uuid-1.jpg");
    expect(store.images[1].thumbnailPath).toBe("/library/thumbs/uuid-2.jpg");
    expect(store.importState).toBe("completed");
  });

  it("deleteImage 从列表中移除对应图片", async () => {
    mockInvoke.mockResolvedValueOnce(mockImages);
    const store = useLibraryStore();
    await store.fetchImages();

    mockInvoke.mockResolvedValueOnce(undefined);
    await store.deleteImage("uuid-1");

    expect(store.images).toHaveLength(1);
    expect(store.images[0].id).toBe("uuid-2");
  });

  it("addFolder 调用 import_entries 并等待返回数量的进度事件后刷新", async () => {
    vi.useFakeTimers();

    mockListen.mockImplementation((_event, handler) => {
      handler({ payload: { id: "uuid-1", status: "completed" } } as Event<unknown>);
      handler({ payload: { id: "uuid-2", status: "completed" } } as Event<unknown>);
      return Promise.resolve(() => {});
    });

    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockInvoke.mockResolvedValueOnce(mockImages.length);

    const store = useLibraryStore();
    const p = store.addFolder("/tmp/memes");

    await vi.runAllTimersAsync();
    await p;
    vi.useRealTimers();

    expect(mockInvoke).toHaveBeenCalledWith("import_entries", {
      entries: [{ kind: "directory", path: "/tmp/memes" }],
    });
    expect(store.images).toEqual(mockImages);
  });

  it("addImages 过程中 indexing 为 true 且 indexCurrent 随进度递增", async () => {
    vi.useFakeTimers();

    let progressHandler!: (e: Event<unknown>) => void;
    mockListen.mockImplementation((_event, handler) => {
      progressHandler = handler as (e: Event<unknown>) => void;
      return Promise.resolve(() => {});
    });

    // import_entries 先挂起，让我们在中途检查状态
    let resolveInvoke!: (v: unknown) => void;
    mockInvoke.mockReturnValueOnce(new Promise((r) => { resolveInvoke = r; }));
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockInvoke.mockResolvedValueOnce(mockImages.length);

    const store = useLibraryStore();
    const p = store.addImages(["/tmp/a.jpg", "/tmp/b.jpg"]);

    await flushPromises();

    expect(store.importState).toBe("preparing");
    resolveInvoke(2);
    await flushPromises();
    expect(store.indexing).toBe(true);
    expect(store.indexTotal).toBe(2);
    expect(store.indexCurrent).toBe(0);
    expect(store.importState).toBe("importing");

    progressHandler({ payload: { id: "uuid-1", status: "completed" } } as Event<unknown>);
    expect(store.indexCurrent).toBe(1);

    progressHandler({ payload: { id: "uuid-2", status: "completed" } } as Event<unknown>);
    expect(store.indexCurrent).toBe(2);

    await vi.runAllTimersAsync();
    await p;
    vi.useRealTimers();

    expect(store.indexing).toBe(false);
    expect(store.importState).toBe("completed");
  });

  it("addFolder 过程中 indexing 为 true 且 indexCurrent 随进度递增", async () => {
    vi.useFakeTimers();

    let progressHandler!: (e: Event<unknown>) => void;
    mockListen.mockImplementation((_event, handler) => {
      progressHandler = handler as (e: Event<unknown>) => void;
      return Promise.resolve(() => {});
    });

    mockInvoke.mockResolvedValueOnce(2);
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockInvoke.mockResolvedValueOnce(mockImages.length);

    const store = useLibraryStore();
    const p = store.addFolder("/tmp/memes");

    await flushPromises();

    expect(store.indexing).toBe(true);
    expect(store.indexTotal).toBe(2);
    expect(store.indexCurrent).toBe(0);
    expect(store.importState).toBe("importing");

    progressHandler({ payload: { id: "uuid-1", status: "completed" } } as Event<unknown>);
    expect(store.indexCurrent).toBe(1);

    progressHandler({ payload: { id: "uuid-2", status: "completed" } } as Event<unknown>);
    expect(store.indexCurrent).toBe(2);

    await vi.runAllTimersAsync();
    await p;
    vi.useRealTimers();

    expect(store.indexing).toBe(false);
    expect(store.importState).toBe("completed");
  });

  it("addFolder 目录为空时（total=0）不监听进度事件", async () => {
    mockInvoke.mockResolvedValueOnce(0);
    const store = useLibraryStore();
    await store.addFolder("/tmp/empty");
    expect(mockListen).not.toHaveBeenCalled();
    expect(store.images).toEqual([]);
    expect(store.importState).toBe("idle");
  });

  it("importEntries 失败时进入 failed 状态", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("boom"));
    const store = useLibraryStore();

    await expect(store.importEntries([{ kind: "file", path: "/tmp/a.jpg" }])).rejects.toThrow("boom");
    expect(store.importState).toBe("failed");
  });

  it("resumeIndexing 可接入后台恢复任务的进度事件", async () => {
    let progressHandler!: (e: Event<unknown>) => void;
    mockListen.mockImplementation((_event, handler) => {
      progressHandler = handler as (e: Event<unknown>) => void;
      return Promise.resolve(() => {});
    });
    mockInvoke.mockResolvedValueOnce(mockImages);
    mockInvoke.mockResolvedValueOnce(2);

    const store = useLibraryStore();
    await store.resumeIndexing(2);

    expect(store.indexing).toBe(true);
    expect(store.indexTotal).toBe(2);
    expect(store.indexCurrent).toBe(0);

    progressHandler({ payload: { id: "uuid-1", status: "completed" } } as Event<unknown>);
    expect(store.indexCurrent).toBe(1);

    progressHandler({ payload: { id: "uuid-2", status: "completed" } } as Event<unknown>);
    await flushPromises();

    expect(store.indexing).toBe(false);
    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 0 });
    expect(mockInvoke).toHaveBeenCalledWith("get_image_count");
  });
});

describe("useLibraryStore 批量选择", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
  });

  it("初始 selectedIds 为空", () => {
    const store = useLibraryStore();
    expect(store.selectedIds.size).toBe(0);
  });

  it("toggleSelection 添加 ID", () => {
    const store = useLibraryStore();
    store.toggleSelection("uuid-1");
    expect(store.selectedIds.has("uuid-1")).toBe(true);
  });

  it("toggleSelection 再次调用移除 ID", () => {
    const store = useLibraryStore();
    store.toggleSelection("uuid-1");
    store.toggleSelection("uuid-1");
    expect(store.selectedIds.has("uuid-1")).toBe(false);
  });

  it("clearSelection 清空所有选中", () => {
    const store = useLibraryStore();
    store.toggleSelection("uuid-1");
    store.toggleSelection("uuid-2");
    store.clearSelection();
    expect(store.selectedIds.size).toBe(0);
  });

  it("deleteSelected 删除选中项并清空选择", async () => {
    mockInvoke.mockResolvedValueOnce(mockImages);
    const store = useLibraryStore();
    await store.fetchImages();

    mockInvoke.mockResolvedValue(undefined);
    store.toggleSelection("uuid-1");
    await store.deleteSelected();

    expect(mockInvoke).toHaveBeenCalledWith("delete_image", { id: "uuid-1" });
    expect(store.images).toHaveLength(1);
    expect(store.images[0].id).toBe("uuid-2");
    expect(store.selectedIds.size).toBe(0);
  });
});

describe("useTaskRecoveryStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockListen.mockResolvedValue(() => {});
  });

  it("支持显式关闭恢复结果摘要", () => {
    const store = useTaskRecoveryStore();
    store.completedRecoverySummary = {
      totalCount: 3,
      importedCount: 2,
      duplicatedCount: 0,
      failedCount: 1,
      failures: [{
        taskId: "task-1",
        fileName: "a.jpg",
        errorMessage: "损坏",
      }],
    };

    store.dismissCompletedRecoverySummary();

    expect(store.completedRecoverySummary).toBeNull();
  });

  it("查看失败项后标记恢复结果已查看并清空摘要", () => {
    const store = useTaskRecoveryStore();
    store.completedRecoverySummary = {
      totalCount: 2,
      importedCount: 1,
      duplicatedCount: 0,
      failedCount: 1,
      failures: [{
        taskId: "task-2",
        fileName: "b.jpg",
        errorMessage: "恢复失败",
      }],
    };

    store.markRecoveryResultSeen();

    expect(store.completedRecoverySummary).toBeNull();
    expect(store.recoveryResultSeen).toBe(true);
  });

  it("新导入开始时清空已确认的恢复结果", () => {
    const recoveryStore = useTaskRecoveryStore();
    recoveryStore.completedRecoverySummary = {
      totalCount: 2,
      importedCount: 2,
      duplicatedCount: 0,
      failedCount: 0,
      failures: [],
    };

    const libraryStore = useLibraryStore();
    libraryStore.importState = "preparing";

    recoveryStore.clearRecoveryResultOnNewImport();

    expect(recoveryStore.completedRecoverySummary).toBeNull();
  });

  it("恢复处理中与普通导入中都暴露统一进行中承接数据", async () => {
    mockInvoke.mockResolvedValueOnce([
      { id: "task-1", filePath: "/tmp/1.jpg", status: "processing" },
      { id: "task-2", filePath: "/tmp/2.jpg", status: "pending" },
      { id: "task-3", filePath: "/tmp/3.jpg", status: "pending" },
    ]);

    const recoveryStore = useTaskRecoveryStore();
    await recoveryStore.fetchPendingTasks(true);

    expect(recoveryStore.inProgressIndicator).toBeNull();

    recoveryStore.activeRecovery = true;
    recoveryStore.recoveryTotal = 3;
    recoveryStore.recoveryImported = 1;
    recoveryStore.recoveryDuplicated = 1;

    expect(recoveryStore.inProgressIndicator).toMatchObject({
      kind: "recovery",
      current: 2,
      total: 3,
      remaining: 1,
    });

    const libraryStore = useLibraryStore();
    libraryStore.indexing = true;
    libraryStore.indexCurrent = 1;
    libraryStore.indexTotal = 4;
    recoveryStore.activeRecovery = false;

    expect(recoveryStore.inProgressIndicator).toMatchObject({
      kind: "import",
      current: 1,
      total: 4,
      remaining: 3,
    });
  });
});
