<template>
  <div
    class="app-shell"
    data-ui-theme="memedesk"
  >
    <GlobalProgressBar />
    <Toast />
    <nav class="nav app-nav">
      <RouterLink
        to="/"
        class="app-nav__link"
      >
        首页 / 搜索
      </RouterLink>
      <RouterLink
        to="/library"
        class="app-nav__link"
      >
        图库管理
      </RouterLink>
      <RouterLink
        to="/settings"
        class="app-nav__link"
      >
        设置
      </RouterLink>
    </nav>
    <main class="app-shell__content">
      <RouterView />
    </main>

    <!-- 任务恢复对话框 -->
    <div
      v-if="recoveryStore.pendingCount > 0"
      class="resume-backdrop ui-dialog-backdrop"
    >
      <div class="resume-dialog ui-dialog">
        <p>上次有 {{ recoveryStore.pendingCount }} 张图片还没整理完。</p>
        <div class="resume-actions">
          <button
            data-action="resume-pending-tasks"
            class="btn-primary ui-button ui-button--primary"
            :disabled="recoveryStore.resuming || recoveryStore.clearing"
            @click="resumeTasks"
          >
            {{ recoveryStore.resuming ? "继续处理中..." : "继续处理" }}
          </button>
          <button
            data-action="clear-pending-tasks"
            class="btn-secondary ui-button ui-button--secondary"
            :disabled="recoveryStore.resuming || recoveryStore.clearing"
            @click="clearPendingTasks"
          >
            {{ recoveryStore.clearing ? "清理中..." : "放弃并清理" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import GlobalProgressBar from "@/components/GlobalProgressBar.vue";
import Toast from "@/components/Toast.vue";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";

const recoveryStore = useTaskRecoveryStore();

onMounted(async () => {
  try {
    await recoveryStore.fetchPendingTasks();
  } catch {
    // 静默失败，不影响正常使用
    console.warn("尝试获取未完成任务时失败");
  }
});

async function resumeTasks() {
  try {
    await recoveryStore.resumePendingTasks();
  } catch {
    // 静默失败
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
