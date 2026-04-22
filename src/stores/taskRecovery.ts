import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useLibraryStore } from "@/stores/library";

interface PendingTask {
  id: string;
  filePath: string;
  status?: string;
}

interface RecoveryFailure {
  taskId: string;
  fileName: string;
  errorMessage?: string;
}

interface RecoverySummary {
  totalCount: number;
  importedCount: number;
  duplicatedCount: number;
  failedCount: number;
  failures: RecoveryFailure[];
}

interface IndexProgressPayload {
  id?: string;
  status?: string;
  resultKind?: string;
  result_kind?: string;
  fileName?: string;
  file_name?: string;
  message?: string | null;
}

export const useTaskRecoveryStore = defineStore("taskRecovery", () => {
  const pendingCount = ref(0);
  const loaded = ref(false);
  const resuming = ref(false);
  const clearing = ref(false);
  const activeRecovery = ref(false);
  const completedRecoverySummary = ref<RecoverySummary | null>(null);
  const recoveryTotal = ref(0);
  const recoveryImported = ref(0);
  const recoveryDuplicated = ref(0);
  const recoveryFailed = ref(0);
  const recoveryFailures = ref<RecoveryFailure[]>([]);
  let progressUnlisten: null | (() => void) = null;

  async function ensureProgressListener() {
    if (progressUnlisten) return;
    progressUnlisten = await listen<IndexProgressPayload>("index-progress", async (event) => {
      if (activeRecovery.value) {
        const payload = event.payload ?? {};
        const resultKind = payload.resultKind ?? payload.result_kind;
        const fileName = payload.fileName ?? payload.file_name ?? "";
        const taskId = payload.id ?? "";
        if (resultKind === "imported") {
          recoveryImported.value += 1;
        } else if (resultKind === "duplicated") {
          recoveryDuplicated.value += 1;
        } else if (resultKind === "failed") {
          recoveryFailed.value += 1;
          recoveryFailures.value = [
            ...recoveryFailures.value,
            {
              taskId,
              fileName,
              errorMessage: payload.message ?? undefined,
            },
          ];
        }
      }
      if (!loaded.value) return;
      await fetchPendingTasks(true);
      if (pendingCount.value <= 0) {
        if (activeRecovery.value && recoveryTotal.value > 0) {
          completedRecoverySummary.value = {
            totalCount: recoveryTotal.value,
            importedCount: recoveryImported.value,
            duplicatedCount: recoveryDuplicated.value,
            failedCount: recoveryFailed.value,
            failures: [...recoveryFailures.value],
          };
        }
        activeRecovery.value = false;
      }
    });
  }

  async function fetchPendingTasks(force = false) {
    if (loaded.value && !force) return;
    await ensureProgressListener();
    const tasks = (await invoke<PendingTask[]>("get_pending_tasks")) ?? [];
    pendingCount.value = tasks.length;
    loaded.value = true;
  }

  async function resumePendingTasks() {
    if (pendingCount.value <= 0 || resuming.value) return;

    const libraryStore = useLibraryStore();
    const total = pendingCount.value;

    resuming.value = true;
    await libraryStore.resumeIndexing(total);
    activeRecovery.value = true;
    completedRecoverySummary.value = null;
    recoveryTotal.value = total;
    recoveryImported.value = 0;
    recoveryDuplicated.value = 0;
    recoveryFailed.value = 0;
    recoveryFailures.value = [];
    try {
      await invoke("resume_pending_tasks");
      await fetchPendingTasks(true);
    } catch (error) {
      activeRecovery.value = false;
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
      activeRecovery.value = false;
      pendingCount.value = 0;
      completedRecoverySummary.value = null;
    } finally {
      clearing.value = false;
    }
  }

  return {
    pendingCount,
    loaded,
    resuming,
    clearing,
    activeRecovery,
    completedRecoverySummary,
    fetchPendingTasks,
    resumePendingTasks,
    clearPendingTasks,
  };
});
