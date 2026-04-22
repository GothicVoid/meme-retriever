import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
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
    failureKind?: string;
    retryable?: boolean;
    userMessage?: string;
}

interface RecoverySummary {
  totalCount: number;
  importedCount: number;
  duplicatedCount: number;
  failedCount: number;
  failures: RecoveryFailure[];
}

interface InProgressIndicator {
  kind: "import" | "recovery";
  current: number;
  total: number;
  remaining: number;
  label: string;
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
  const libraryStore = useLibraryStore();
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
  const recoveryResultSeen = ref(false);
  let progressUnlisten: null | (() => void) = null;
  let progressListenerPromise: Promise<void> | null = null;

  const inProgressIndicator = computed<InProgressIndicator | null>(() => {
    if (activeRecovery.value && recoveryTotal.value > 0) {
      const current = Math.min(
        recoveryTotal.value,
        Math.max(
          recoveryImported.value + recoveryDuplicated.value + recoveryFailed.value,
          libraryStore.indexCurrent
        )
      );
      const remaining = Math.max(0, recoveryTotal.value - current);
      return {
        kind: "recovery",
        current,
        total: recoveryTotal.value,
        remaining,
        label: `正在继续导入 ${current}/${recoveryTotal.value}`,
      };
    }

    if (libraryStore.indexing && libraryStore.indexTotal > 0) {
      const current = Math.min(libraryStore.indexCurrent, libraryStore.indexTotal);
      const remaining = Math.max(0, libraryStore.indexTotal - current);
      return {
        kind: "import",
        current,
        total: libraryStore.indexTotal,
        remaining,
        label: `正在导入 ${current}/${libraryStore.indexTotal}`,
      };
    }

    return null;
  });

  const shouldShowRecoveryDialog = computed(() =>
    pendingCount.value >= 3 && !activeRecovery.value && !libraryStore.indexing
  );

  const shouldShowRecoveryFailureNudge = computed(() =>
    !!completedRecoverySummary.value
      && completedRecoverySummary.value.failedCount > 0
      && !recoveryResultSeen.value
  );

  async function ensureProgressListener() {
    if (progressUnlisten) return;
    if (progressListenerPromise) {
      await progressListenerPromise;
      return;
    }

    progressListenerPromise = (async () => {
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
            recoveryResultSeen.value = false;
          }
          activeRecovery.value = false;
        }
      });
    })();

    try {
      await progressListenerPromise;
    } finally {
      progressListenerPromise = null;
    }
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
    recoveryResultSeen.value = false;
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

  function dismissCompletedRecoverySummary() {
    completedRecoverySummary.value = null;
  }

  function markRecoveryResultSeen() {
    recoveryResultSeen.value = true;
    dismissCompletedRecoverySummary();
  }

  function clearRecoveryResultOnNewImport() {
    dismissCompletedRecoverySummary();
  }

  watch(
    () => libraryStore.importState,
    (state) => {
      if (state === "preparing" || state === "importing") {
        clearRecoveryResultOnNewImport();
      }
    }
  );

  return {
    pendingCount,
    loaded,
    resuming,
    clearing,
    activeRecovery,
    completedRecoverySummary,
    recoveryTotal,
    recoveryImported,
    recoveryDuplicated,
    recoveryFailed,
    recoveryFailures,
    recoveryResultSeen,
    inProgressIndicator,
    shouldShowRecoveryDialog,
    shouldShowRecoveryFailureNudge,
    fetchPendingTasks,
    resumePendingTasks,
    clearPendingTasks,
    dismissCompletedRecoverySummary,
    markRecoveryResultSeen,
    clearRecoveryResultOnNewImport,
  };
});
