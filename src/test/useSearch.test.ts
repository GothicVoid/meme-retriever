import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSearch } from "@/composables/useSearch";

const mockInvoke = vi.mocked(invoke);

describe("useSearch", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it("返回 store 和 debouncedSearch", () => {
    const { store, debouncedSearch } = useSearch();
    expect(store).toBeDefined();
    expect(typeof debouncedSearch).toBe("function");
  });

  it("debouncedSearch 延迟后调用 store.search", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue([]);

    const { debouncedSearch } = useSearch();
    debouncedSearch("hello");

    // 300ms 前不应调用
    expect(mockInvoke).not.toHaveBeenCalled();

    // 推进 300ms
    await vi.advanceTimersByTimeAsync(300);
    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "hello" }));

    vi.useRealTimers();
  });

  it("快速连续输入只触发一次搜索", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue([]);

    const { debouncedSearch } = useSearch();
    debouncedSearch("a");
    debouncedSearch("ab");
    debouncedSearch("abc");

    await vi.advanceTimersByTimeAsync(300);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({ query: "abc" }));

    vi.useRealTimers();
  });
});
