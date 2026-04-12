import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useClipboard } from "@/composables/useClipboard";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);

describe("useClipboard", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("copyImage 调用 copy_to_clipboard 命令", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const { copyImage } = useClipboard();

    await copyImage("img-1");

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("copy_to_clipboard", { id: "img-1" });
  });
});
