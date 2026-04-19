<template>
  <div
    class="app-shell"
    :class="`app-shell--${effectiveWindowMode}`"
    data-ui-theme="memedesk"
  >
    <GlobalProgressBar />
    <Toast />

    <header
      v-if="isSidebarMode"
      class="app-shell__topbar"
    >
      <div class="app-shell__brand">
        <span class="app-shell__brand-mark">M</span>
        <div class="app-shell__brand-copy">
          <strong>快速找图</strong>
        </div>
      </div>
      <div class="app-shell__menu-wrap">
        <button
          ref="moreButtonRef"
          type="button"
          class="app-shell__menu-button"
          data-action="toggle-more-menu"
          aria-label="打开整理和设置"
          @click="toggleMoreMenu"
        >
          整理
        </button>
        <div
          v-if="showMoreMenu"
          ref="moreMenuRef"
          class="app-shell__more-menu ui-floating-panel"
        >
          <button
            type="button"
            class="app-shell__more-action"
            data-action="open-gallery-management"
            @click="openGalleryManagement"
          >
            整理图库
          </button>
          <button
            type="button"
            class="app-shell__more-action"
            data-action="open-settings"
            @click="openSettingsPanel"
          >
            打开设置
          </button>
          <button
            type="button"
            class="app-shell__more-action"
            data-action="toggle-dock-side"
            @click="toggleDockSide"
          >
            {{ settings.dockSide === "right" ? "切到左侧" : "切到右侧" }}
          </button>
          <button
            type="button"
            class="app-shell__more-action"
            data-action="enter-expanded-mode"
            @click="enterExpandedManagement"
          >
            展开整理模式
          </button>
        </div>
      </div>
    </header>

    <header
      v-else
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RouterView, useRoute, useRouter } from "vue-router";
import GlobalProgressBar from "@/components/GlobalProgressBar.vue";
import Toast from "@/components/Toast.vue";
import { useSettingsStore, type WindowMode } from "@/stores/settings";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";
import { applyWindowLayout } from "@/utils/windowLayout";

const recoveryStore = useTaskRecoveryStore();
const settings = useSettingsStore();
const route = useRoute();
const router = useRouter();

const showMoreMenu = ref(false);
const moreMenuRef = ref<HTMLElement | null>(null);
const moreButtonRef = ref<HTMLElement | null>(null);

const isSidebarMode = computed(() =>
  route.path === "/" && settings.windowMode === "sidebar"
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

function closeMoreMenu() {
  showMoreMenu.value = false;
}

function toggleMoreMenu() {
  showMoreMenu.value = !showMoreMenu.value;
}

async function goQuickSearch() {
  settings.windowMode = "sidebar";
  closeMoreMenu();
  if (route.path !== "/") {
    await router.push("/");
  }
}

async function openGalleryManagement() {
  settings.windowMode = "expanded";
  closeMoreMenu();
  if (route.path !== "/library") {
    await router.push("/library");
  }
}

async function openSettingsPanel() {
  settings.windowMode = "expanded";
  closeMoreMenu();
  if (route.path !== "/settings") {
    await router.push("/settings");
  }
}

async function enterExpandedManagement() {
  await openGalleryManagement();
}

function toggleDockSide() {
  settings.dockSide = settings.dockSide === "right" ? "left" : "right";
  closeMoreMenu();
}

function handlePointerDown(event: MouseEvent) {
  const target = event.target as Node | null;
  if (
    showMoreMenu.value
    && target
    && !moreMenuRef.value?.contains(target)
    && !moreButtonRef.value?.contains(target)
  ) {
    closeMoreMenu();
  }
}

onMounted(async () => {
  try {
    await recoveryStore.fetchPendingTasks();
  } catch {
    console.warn("尝试获取未完成任务时失败");
  }

  document.addEventListener("mousedown", handlePointerDown);
});

onBeforeUnmount(() => {
  document.removeEventListener("mousedown", handlePointerDown);
});

watch(
  [effectiveWindowMode, () => settings.dockSide, () => route.fullPath],
  async ([mode, dockSide]) => {
    await nextTick();
    await applyWindowLayout(mode, dockSide);
  },
  { immediate: true }
);

watch(() => route.fullPath, () => {
  closeMoreMenu();
});

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
.app-shell__topbar,
.app-shell__expanded-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.app-shell__brand {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}

.app-shell__brand-mark {
  width: 2rem;
  height: 2rem;
  border-radius: 0.85rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(183, 121, 31, 0.15);
  color: var(--ui-accent);
  font-weight: 800;
}

.app-shell__brand-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.app-shell__brand-copy strong {
  font-size: 0.95rem;
}

.app-shell__menu-wrap {
  position: relative;
}

.app-shell__menu-button,
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

.app-shell__menu-button {
  min-height: 2.25rem;
  padding: 0 1rem;
  font-size: 0.84rem;
  font-weight: 600;
}

.app-shell__menu-button:hover,
.app-shell__toolbar-action:hover {
  background: var(--ui-bg-hover);
  border-color: var(--ui-border-strong);
}

.app-shell__more-menu {
  position: absolute;
  top: calc(100% + 0.5rem);
  right: 0;
  width: 12rem;
  padding: 0.4rem;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  z-index: 30;
}

.app-shell__more-action {
  width: 100%;
  padding: 0.65rem 0.8rem;
  border: none;
  border-radius: 0.85rem;
  background: transparent;
  color: var(--ui-text-primary);
  text-align: left;
  cursor: pointer;
}

.app-shell__more-action:hover {
  background: var(--ui-bg-hover);
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
