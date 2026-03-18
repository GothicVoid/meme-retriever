// mock @tauri-apps/api/core 的 invoke
import { vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
