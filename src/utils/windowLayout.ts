import { invoke } from "@tauri-apps/api/core";
import type { DockSide, WindowMode } from "@/stores/settings";

export async function applyWindowLayout(mode: WindowMode, dockSide: DockSide) {
  try {
    await invoke("apply_window_layout", { mode, dockSide });
  } catch (error) {
    console.warn("apply_window_layout failed:", error);
  }
}

export async function saveWindowPreferences(mode: WindowMode, dockSide: DockSide) {
  try {
    await invoke("save_window_preferences", { mode, dockSide });
  } catch (error) {
    console.warn("save_window_preferences failed:", error);
  }
}
