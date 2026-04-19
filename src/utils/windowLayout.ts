import { invoke } from "@tauri-apps/api/core";
import type { WindowMode } from "@/stores/settings";

export async function applyWindowLayout(mode: WindowMode) {
  try {
    await invoke("apply_window_layout", { mode });
  } catch (error) {
    console.warn("apply_window_layout failed:", error);
  }
}

export async function saveWindowPreferences(mode: WindowMode) {
  try {
    await invoke("save_window_preferences", { mode });
  } catch (error) {
    console.warn("save_window_preferences failed:", error);
  }
}
