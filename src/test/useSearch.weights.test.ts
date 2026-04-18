import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSearch } from "@/composables/useSearch";

const mockInvoke = vi.mocked(invoke);

describe("useSearch — 不再透传权重", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    localStorage.clear();
  });

  it("debouncedSearch 仅透传 query 和默认 limit", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue([]);

    const { debouncedSearch } = useSearch();
    debouncedSearch("hello");
    await vi.advanceTimersByTimeAsync(300);

    expect(mockInvoke).toHaveBeenCalledWith("search", { query: "hello", limit: 30 });

    vi.useRealTimers();
  });
});
