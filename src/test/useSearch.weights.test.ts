import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSearch } from "@/composables/useSearch";
import { useSettingsStore } from "@/stores/settings";

const mockInvoke = vi.mocked(invoke);

describe("useSearch — 权重从 settings 传入", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    localStorage.clear();
  });

  it("debouncedSearch 使用 settings 中的归一化权重", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue([]);

    const settings = useSettingsStore();
    settings.w1 = 1;
    settings.w2 = 1;
    settings.w3 = 0;

    const { debouncedSearch } = useSearch();
    debouncedSearch("hello");
    await vi.advanceTimersByTimeAsync(300);

    expect(mockInvoke).toHaveBeenCalledWith("search", expect.objectContaining({
      w1: expect.closeTo(0.5, 3),
      w2: expect.closeTo(0.5, 3),
      w3: expect.closeTo(0, 3),
    }));

    vi.useRealTimers();
  });

  it("默认权重下 w1+w2+w3 传入值之和为 1", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue([]);

    const { debouncedSearch } = useSearch();
    debouncedSearch("test");
    await vi.advanceTimersByTimeAsync(300);

    const call = mockInvoke.mock.calls[0][1] as { w1: number; w2: number; w3: number };
    expect(call.w1 + call.w2 + call.w3).toBeCloseTo(1, 5);

    vi.useRealTimers();
  });
});
