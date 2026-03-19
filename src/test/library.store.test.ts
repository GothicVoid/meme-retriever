import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type { Event } from "@tauri-apps/api/event";
import { useLibraryStore, type ImageMeta } from "@/stores/library";

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
  });

  it("fetchImages 调用 invoke 并更新 images 列表", async () => {
    mockInvoke.mockResolvedValueOnce(mockImages);

    const store = useLibraryStore();
    await store.fetchImages();

    expect(mockInvoke).toHaveBeenCalledWith("get_images", { page: 0 });
    expect(store.images).toEqual(mockImages);
    expect(store.loading).toBe(false);
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

  it("addImages 完成后图库列表包含新增图片，thumbnailPath 正确", async () => {
    vi.useFakeTimers();

    // listen 立即触发回调，模拟两张图片入库完成
    mockListen.mockImplementation((_event, handler) => {
      handler({ payload: { id: "uuid-1", status: "completed" } } as Event<unknown>);
      handler({ payload: { id: "uuid-2", status: "completed" } } as Event<unknown>);
      return Promise.resolve(() => {});
    });

    // add_images 立即 resolve，get_images 返回 mockImages
    mockInvoke.mockResolvedValueOnce(undefined);
    mockInvoke.mockResolvedValueOnce(mockImages);

    const store = useLibraryStore();
    const p = store.addImages(["/tmp/sample.jpg", "/tmp/sample_blank.jpg"]);

    // 推进 polling interval
    await vi.runAllTimersAsync();
    await p;

    vi.useRealTimers();

    expect(store.images).toEqual(mockImages);
    expect(store.images[0].thumbnailPath).toBe("/library/thumbs/uuid-1.jpg");
    expect(store.images[1].thumbnailPath).toBe("/library/thumbs/uuid-2.jpg");
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
});
