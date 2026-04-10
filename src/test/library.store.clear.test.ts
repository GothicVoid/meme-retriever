import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import type { Event } from "@tauri-apps/api/event";
import { flushPromises } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { listen } from "@tauri-apps/api/event";
import { useLibraryStore, type ImageMeta } from "@/stores/library";

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

const mockImages: ImageMeta[] = [
  {
    id: "uuid-1",
    filePath: "/library/images/uuid-1.jpg",
    fileName: "sample.jpg",
    thumbnailPath: "/library/thumbs/uuid-1.jpg",
    fileFormat: "jpg",
    width: 800,
    height: 600,
    fileSize: 123,
    addedAt: 1700000000,
    useCount: 0,
    tags: [],
  },
  {
    id: "uuid-2",
    filePath: "/library/images/uuid-2.jpg",
    fileName: "sample2.jpg",
    thumbnailPath: "/library/thumbs/uuid-2.jpg",
    fileFormat: "jpg",
    width: 400,
    height: 400,
    fileSize: 456,
    addedAt: 1700000001,
    useCount: 0,
    tags: [],
  },
];

describe("useLibraryStore clearGallery", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockListen.mockReset();
  });

  it("图库为空时不调用 clear_gallery", async () => {
    const store = useLibraryStore();
    await store.clearGallery();
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(mockListen).not.toHaveBeenCalled();
  });

  it("调用 clear_gallery 并在完成后清空图片与选择", async () => {
    vi.useFakeTimers();

    let progressHandler!: (event: Event<unknown>) => void;
    const unlisten = vi.fn();
    mockListen.mockImplementation((_event, handler) => {
      progressHandler = handler as (event: Event<unknown>) => void;
      return Promise.resolve(unlisten);
    });

    let resolveInvoke!: (value: unknown) => void;
    mockInvoke.mockReturnValueOnce(new Promise((resolve) => {
      resolveInvoke = resolve;
    }));

    const store = useLibraryStore();
    store.images = [...mockImages];
    store.toggleSelection("uuid-1");

    const promise = store.clearGallery();
    await flushPromises();

    expect(store.clearing).toBe(true);
    expect(store.clearTotal).toBe(2);
    expect(store.clearCurrent).toBe(0);
    expect(mockInvoke).toHaveBeenCalledWith("clear_gallery");

    progressHandler({ payload: { current: 1, total: 2 } } as Event<unknown>);
    expect(store.clearCurrent).toBe(1);
    expect(store.clearTotal).toBe(2);

    progressHandler({ payload: { current: 2, total: 2 } } as Event<unknown>);
    resolveInvoke(undefined);

    await vi.runAllTimersAsync();
    await promise;
    vi.useRealTimers();

    expect(store.clearing).toBe(false);
    expect(store.images).toEqual([]);
    expect(store.selectedIds.size).toBe(0);
    expect(unlisten).toHaveBeenCalled();
  });

  it("进度事件会更新 clearCurrent 和 clearTotal", async () => {
    vi.useFakeTimers();

    let progressHandler!: (event: Event<unknown>) => void;
    mockListen.mockImplementation((_event, handler) => {
      progressHandler = handler as (event: Event<unknown>) => void;
      return Promise.resolve(() => {});
    });
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useLibraryStore();
    store.images = [...mockImages];

    const promise = store.clearGallery();
    await flushPromises();

    progressHandler({ payload: { current: 1, total: 5 } } as Event<unknown>);
    expect(store.clearCurrent).toBe(1);
    expect(store.clearTotal).toBe(5);

    progressHandler({ payload: { current: 5, total: 5 } } as Event<unknown>);
    await vi.runAllTimersAsync();
    await promise;
    vi.useRealTimers();
  });
});
