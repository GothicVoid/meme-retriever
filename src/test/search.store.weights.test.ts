import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSearchStore } from "@/stores/search";

const mockInvoke = vi.mocked(invoke);

describe("useSearchStore — 权重参数", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("search 不传权重时 invoke 参数含 undefined w1/w2/w3", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const store = useSearchStore();
    await store.search("test");
    expect(mockInvoke).toHaveBeenCalledWith("search", {
      query: "test",
      limit: 9,
      w1: undefined,
      w2: undefined,
      w3: undefined,
    });
  });

  it("search 传入权重时正确透传给 invoke", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const store = useSearchStore();
    await store.search("test", 9, 0.5, 0.3, 0.2);
    expect(mockInvoke).toHaveBeenCalledWith("search", {
      query: "test",
      limit: 9,
      w1: 0.5,
      w2: 0.3,
      w3: 0.2,
    });
  });

  it("search 传入自定义 limit", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const store = useSearchStore();
    await store.search("test", 21, 0.3, 0.4, 0.3);
    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ limit: 21 }));
  });
});
