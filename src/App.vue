<template>
  <div
    class="app-shell"
    :class="`app-shell--${effectiveWindowMode}`"
    data-ui-theme="memedesk"
  >
    <GlobalProgressBar />
    <Toast />

    <header
      v-if="!isSidebarMode"
      class="app-shell__expanded-toolbar"
    >
      <div class="app-shell__expanded-title">
        <h1>{{ expandedTitle }}</h1>
        <p>{{ expandedSubtitle }}</p>
      </div>
      <div class="app-shell__expanded-actions">
        <button
          type="button"
          class="app-shell__toolbar-action"
          :class="{ 'app-shell__toolbar-action--active': route.path === '/' }"
          data-action="go-search"
          @click="goQuickSearch"
        >
          快速找图
        </button>
        <button
          type="button"
          class="app-shell__toolbar-action"
          :class="{ 'app-shell__toolbar-action--active': route.path === '/library' }"
          data-action="go-library"
          @click="openGalleryManagement"
        >
          整理图库
        </button>
        <button
          type="button"
          class="app-shell__toolbar-action"
          :class="{ 'app-shell__toolbar-action--active': route.path === '/settings' }"
          data-action="go-settings"
          @click="openSettingsPanel"
        >
          设置
        </button>
      </div>
    </header>

    <main
      class="app-shell__content"
      :class="{ 'app-shell__content--sidebar': isSidebarMode }"
    >
      <RouterView />
    </main>

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
import { computed, nextTick, onMounted, watch } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";
import GlobalProgressBar from "@/components/GlobalProgressBar.vue";
import Toast from "@/components/Toast.vue";
import { useSettingsStore, type WindowMode } from "@/stores/settings";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";
import { applyWindowLayout, saveWindowPreferences } from "@/utils/windowLayout";

const recoveryStore = useTaskRecoveryStore();
const settings = useSettingsStore();
const route = useRoute();
const router = useRouter();

const isSidebarMode = computed(() =>
  route.path === "/" && settings.currentWindowMode === "sidebar"
);

const effectiveWindowMode = computed<WindowMode>(() =>
  isSidebarMode.value ? "sidebar" : "expanded"
);

const expandedTitle = computed(() => {
  if (route.path === "/settings") {
    return "设置与维护";
  }
  if (route.path === "/library") {
    return "图库整理";
  }
  return "展开工作台";
});

const expandedSubtitle = computed(() => {
  if (route.path === "/settings") {
    return "窗口偏好、调试能力和索引维护都放在这里。";
  }
  if (route.path === "/library") {
    return "导入、排查、批量整理等低频重任务在这里处理。";
  }
  return "需要更大工作区时，再离开聊天伴随态。";
});

async function goQuickSearch() {
  settings.currentWindowMode = "sidebar";
  if (route.path !== "/") {
    await router.push("/");
  }
}

async function openGalleryManagement() {
  settings.currentWindowMode = "expanded";
  if (route.path !== "/library") {
    await router.push("/library");
  }
}

async function openSettingsPanel() {
  settings.currentWindowMode = "expanded";
  if (route.path !== "/settings") {
    await router.push("/settings");
  }
}

onMounted(async () => {
  try {
    await recoveryStore.fetchPendingTasks();
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
.app-shell__expanded-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.app-shell__toolbar-action {
  border: 1px solid var(--ui-border-subtle);
  border-radius: 999px;
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 90%, white);
  color: var(--ui-text-primary);
  cursor: pointer;
  transition:
    background-color 120ms ease,
    border-color 120ms ease,
    color 120ms ease;
}

.app-shell__toolbar-action:hover {
  background: var(--ui-bg-hover);
  border-color: var(--ui-border-strong);
}

.app-shell__expanded-title {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.app-shell__expanded-title h1 {
  margin: 0;
  font-size: 1.2rem;
}

.app-shell__expanded-title p {
  margin: 0;
  font-size: 0.84rem;
  color: var(--ui-text-secondary);
}

.app-shell__expanded-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.app-shell__toolbar-action {
  min-height: 2.4rem;
  padding: 0 1rem;
  font-size: 0.9rem;
}

.app-shell__toolbar-action--active {
  background: rgba(183, 121, 31, 0.12);
  border-color: var(--ui-border-strong);
  color: var(--ui-accent);
  font-weight: 600;
}

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
