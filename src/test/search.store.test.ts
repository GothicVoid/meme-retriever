import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSearchStore } from "@/stores/search";

const mockInvoke = vi.mocked(invoke);

describe("useSearchStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("初始状态正确", () => {
    const store = useSearchStore();
    expect(store.query).toBe("");
    expect(store.results).toEqual([]);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("search 调用 invoke 并更新 results", async () => {
    const mockResults = [
      { id: "1", filePath: "/tmp/a.jpg", thumbnailPath: "/tmp/a_thumb.jpg", score: 0.9, tags: ["搞笑"] },
    ];
    mockInvoke.mockResolvedValueOnce(mockResults);

    const store = useSearchStore();
    await store.search("蚌埠住了");

    expect(mockInvoke).toHaveBeenCalledWith("search", { query: "蚌埠住了", limit: 9 });
    expect(store.results).toEqual(mockResults);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("search 失败时设置 error", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("search failed"));

    const store = useSearchStore();
    await store.search("test");

    expect(store.results).toEqual([]);
    expect(store.error).toContain("search failed");
    expect(store.loading).toBe(false);
  });

  it("search 过程中 loading 为 true", async () => {
    let resolveSearch!: (v: unknown) => void;
    mockInvoke.mockReturnValueOnce(new Promise((r) => { resolveSearch = r; }));

    const store = useSearchStore();
    const searchPromise = store.search("test");
    expect(store.loading).toBe(true);

    resolveSearch([]);
    await searchPromise;
    expect(store.loading).toBe(false);
  });
});
