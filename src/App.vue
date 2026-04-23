<template>
  <div
    class="app-shell"
    :class="`app-shell--${effectiveWindowMode}`"
    data-ui-theme="memedesk"
  >
    <GlobalProgressBar />
    <Toast />

    <main
      class="app-shell__content"
      :class="{
        'app-shell__content--sidebar': isSidebarMode,
        'app-shell__content--library': route.path === '/library',
      }"
    >
      <RouterView />
    </main>

    <div
      v-if="showRecoveryDialog"
      class="resume-backdrop ui-dialog-backdrop"
    >
      <div class="resume-dialog ui-dialog">
        <p>上次导入中断，还有 {{ recoveryStore.pendingCount }} 张图片未处理。</p>
        <div class="resume-actions">
          <button
            data-action="resume-pending-tasks"
            class="btn-primary ui-button ui-button--primary"
            :disabled="recoveryStore.resuming || recoveryStore.clearing"
            @click="resumeTasks"
          >
            {{ recoveryStore.resuming ? "继续导入中..." : "继续导入" }}
          </button>
          <button
            data-action="clear-pending-tasks"
            class="btn-secondary ui-button ui-button--secondary"
            :disabled="recoveryStore.resuming || recoveryStore.clearing"
            @click="clearPendingTasks"
          >
            {{ recoveryStore.clearing ? "放弃中..." : "放弃剩余图片" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, watch } from "vue";
import { RouterView, useRoute } from "vue-router";
import GlobalProgressBar from "@/components/GlobalProgressBar.vue";
import Toast from "@/components/Toast.vue";
import { useSettingsStore, type WindowMode } from "@/stores/settings";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";
import { applyWindowLayout, saveWindowPreferences } from "@/utils/windowLayout";

const recoveryStore = useTaskRecoveryStore();
const settings = useSettingsStore();
const route = useRoute();
const isSidebarMode = computed(() =>
  route.path === "/" && settings.currentWindowMode === "sidebar"
);

const effectiveWindowMode = computed<WindowMode>(() =>
  isSidebarMode.value ? "sidebar" : "expanded"
);

const showRecoveryDialog = computed(() =>
  recoveryStore.shouldShowRecoveryDialog && route.path !== "/library"
);

onMounted(async () => {
  try {
    await recoveryStore.fetchPendingTasks(true);
  } catch {
    console.warn("尝试获取未完成任务时失败");
  }
});

watch(
  [effectiveWindowMode, () => route.fullPath],
  async ([mode]) => {
    await nextTick();
    await Promise.all([
      applyWindowLayout(mode),
      saveWindowPreferences(mode),
    ]);
  },
  { immediate: true }
);

async function resumeTasks() {
  try {
    await recoveryStore.resumePendingTasks();
  } catch {
    console.warn("尝试恢复未完成任务时失败");
  }
}

async function clearPendingTasks() {
  try {
    await recoveryStore.clearPendingTasks();
  } catch {
    console.warn("尝试清理未完成任务时失败");
  }
}
</script>

<style scoped>
.resume-dialog p {
  font-size: 0.95rem;
  line-height: 1.6;
}

.resume-actions {
  display: flex;
  gap: 0.75rem;
  justify-content: flex-end;
}
</style>
