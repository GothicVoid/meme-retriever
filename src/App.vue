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
      v-if="showResumeDialog"
      class="resume-backdrop ui-dialog-backdrop"
      @click.self="showResumeDialog = false"
    >
      <div class="resume-dialog ui-dialog">
        <p>检测到 {{ pendingCount }} 个未完成的入库任务，是否继续？</p>
        <div class="resume-actions">
          <button
            class="btn-primary ui-button ui-button--primary"
            @click="resumeTasks"
          >
            继续入库
          </button>
          <button
            class="btn-secondary ui-button ui-button--secondary"
            @click="dismissResume"
          >
            忽略
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import GlobalProgressBar from "@/components/GlobalProgressBar.vue";
import Toast from "@/components/Toast.vue";

const showResumeDialog = ref(false);
const pendingCount = ref(0);

onMounted(async () => {
  try {
    const tasks = await invoke<{ id: number; filePath: string }[]>("get_pending_tasks");
    if (tasks.length > 0) {
      pendingCount.value = tasks.length;
      showResumeDialog.value = true;
    }
  } catch {
    // 静默失败，不影响正常使用
    console.warn('尝试获取未完成任务时失败')
  }
});

async function resumeTasks() {
  showResumeDialog.value = false;
  try {
    await invoke("resume_pending_tasks");
  } catch {
    // 静默失败
    console.warn('尝试恢复未完成任务时失败')
  }
}

function dismissResume() {
  showResumeDialog.value = false;
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
