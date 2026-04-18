<template>
  <GlobalProgressBar />
  <Toast />
  <nav class="nav">
    <RouterLink to="/">
      首页 / 搜索
    </RouterLink>
    <RouterLink to="/library">
      图库管理
    </RouterLink>
    <RouterLink to="/settings">
      设置
    </RouterLink>
  </nav>
  <RouterView />

  <!-- 任务恢复对话框 -->
  <div
    v-if="showResumeDialog"
    class="resume-backdrop"
    @click.self="showResumeDialog = false"
  >
    <div class="resume-dialog">
      <p>检测到 {{ pendingCount }} 个未完成的入库任务，是否继续？</p>
      <div class="resume-actions">
        <button
          class="btn-primary"
          @click="resumeTasks"
        >
          继续入库
        </button>
        <button
          class="btn-secondary"
          @click="dismissResume"
        >
          忽略
        </button>
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

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  color: #0f0f0f;
  background-color: #f6f6f6;
}
@media (prefers-color-scheme: dark) {
  :root { color: #f6f6f6; background-color: #2f2f2f; }
}
* { box-sizing: border-box; margin: 0; padding: 0; }
</style>

<style scoped>
.nav {
  display: flex;
  gap: 1rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #eee;
}
.nav a { text-decoration: none; color: #555; }
.nav a.router-link-active { color: #000; font-weight: 600; }

.resume-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}
.resume-dialog {
  background: #fff;
  border-radius: 10px;
  padding: 1.5rem 2rem;
  max-width: 360px;
  width: 90vw;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  box-shadow: 0 8px 24px rgba(0,0,0,0.15);
}
.resume-dialog p { font-size: 0.95rem; line-height: 1.5; }
.resume-actions { display: flex; gap: 0.75rem; justify-content: flex-end; }
.btn-primary {
  padding: 0.45rem 1.1rem;
  background: #646cff;
  color: #fff;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}
.btn-primary:hover { background: #535bf2; }
.btn-secondary {
  padding: 0.45rem 1.1rem;
  background: none;
  border: 1px solid #ccc;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
  color: #555;
}
.btn-secondary:hover { background: #f5f5f5; }
</style>
