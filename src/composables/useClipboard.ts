import { invoke } from "@tauri-apps/api/core";

export function useClipboard() {
  async function copyImage(id: string) {
    await invoke("copy_to_clipboard", { id });
  }
  return { copyImage };
}
