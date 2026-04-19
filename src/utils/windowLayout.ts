import { invoke } from "@tauri-apps/api/core";
import type { DockSide, WindowMode } from "@/stores/settings";

export async function applyWindowLayout(mode: WindowMode, dockSide: DockSide) {
  try {
    await invoke("apply_window_layout", { mode, dockSide });
  } catch (error) {
    console.warn("apply_window_layout failed:", error);
  }
}

export async function showMainWindow() {
  try {
    await invoke("show_main_window");
  } catch (error) {
    console.warn("show_main_window failed:", error);
  }
}
