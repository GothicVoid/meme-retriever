import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSearchStore } from "@/stores/search";

const mockInvoke = vi.mocked(invoke);

describe("useSearchStore — 搜索接口已移除权重参数", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("search 调用 invoke 时不再传 w1/w2/w3", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const store = useSearchStore();
    await store.search("test");
    expect(mockInvoke).toHaveBeenCalledWith("search", {
      query: "test",
      limit: 9,
    });
  });

  it("search 传入自定义 limit", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    const store = useSearchStore();
    await store.search("test", 21);
    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ limit: 21 }));
  });
});
