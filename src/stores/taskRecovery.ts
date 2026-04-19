import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useLibraryStore } from "@/stores/library";

interface PendingTask {
  id: number;
  filePath: string;
}

export const useTaskRecoveryStore = defineStore("taskRecovery", () => {
  const pendingCount = ref(0);
  const loaded = ref(false);
  const resuming = ref(false);
  const clearing = ref(false);

  async function fetchPendingTasks(force = false) {
    if (loaded.value && !force) return;
    const tasks = await invoke<PendingTask[]>("get_pending_tasks");
    pendingCount.value = tasks.length;
    loaded.value = true;
  }

  async function resumePendingTasks() {
    if (pendingCount.value <= 0 || resuming.value) return;

    const libraryStore = useLibraryStore();
    const total = pendingCount.value;

    resuming.value = true;
    await libraryStore.resumeIndexing(total);
    try {
      await invoke("resume_pending_tasks");
      pendingCount.value = 0;
    } catch (error) {
      libraryStore.stopResumeIndexing();
      throw error;
    } finally {
      resuming.value = false;
    }
  }

  async function clearPendingTasks() {
    if (pendingCount.value <= 0 || clearing.value) return;

    clearing.value = true;
    try {
      await invoke("clear_task_queue");
      pendingCount.value = 0;
    } finally {
      clearing.value = false;
    }
  }

  return {
    pendingCount,
    loaded,
    resuming,
    clearing,
    fetchPendingTasks,
    resumePendingTasks,
    clearPendingTasks,
  };
});
