<template>
  <div class="kb-view">
    <header class="topbar">
      <button
        type="button"
        class="ghost-btn small"
        data-action="back-to-library"
        @click="goBack"
      >
        返回图库
      </button>

      <div class="topbar-copy">
        <p class="eyebrow">角色搜索增强</p>
        <h1>按角色名搜不到时，补几张图试一下，能搜到再保存</h1>
      </div>

      <div class="topbar-actions">
        <button
          class="primary-btn small"
          data-action="save-kb"
          :disabled="loading || saving"
          @click="saveKnowledgeBase"
        >
          {{ saving ? "保存中..." : "保存角色库" }}
        </button>
      </div>
    </header>

    <div class="meta-row">
      <span class="meta-pill">角色数：{{ entries.length }}</span>
      <span
        class="meta-pill"
        :class="{ dirty: dirty }"
      >
        {{ dirty ? "有未保存修改" : "已与磁盘同步" }}
      </span>
    </div>

    <p
      v-if="statusMessage"
      class="status-line"
    >
      {{ statusMessage }}
    </p>

    <div class="workspace">
      <aside class="entry-rail">
        <div class="entry-panel">
          <div class="panel-head">
            <div>
              <h2>角色</h2>
              <p class="panel-copy">已有就直接选，没有再新建。</p>
            </div>
            <button
              class="ghost-btn small"
              data-action="new-entry"
              @click="createEntry"
            >
              新建
            </button>
          </div>

          <input
            v-model.trim="filterKeyword"
            class="filter-input"
            type="text"
            placeholder="按角色名 / 别名 / 线索词筛选"
          >

          <div class="entry-list">
            <button
              v-for="entry in filteredEntries"
              :key="entry.id"
              class="entry-item"
              :class="{ active: entry.id === selectedEntryId }"
              :data-entry="entry.name"
              @click="selectEntry(entry.id)"
            >
              <span class="entry-title">{{ entry.name || "未命名角色" }}</span>
              <span class="entry-meta">{{ entry.exampleImages.length > 0 ? "已配示例图" : "缺少示例图" }}</span>
            </button>
            <div
              v-if="filteredEntries.length === 0"
              class="empty-state"
            >
              当前筛选下没有角色
            </div>
          </div>
        </div>
      </aside>

      <section class="content-stack">
        <section
          v-if="selectedEntry"
          class="editor-shell"
        >
          <div class="editor-head">
            <div>
              <h2>当前角色</h2>
              <p class="panel-copy">先补名字和示例图，再试一下能不能搜到。</p>
            </div>
            <button
              class="danger-btn small"
              data-action="delete-entry"
              :disabled="!selectedEntry"
              @click="deleteCurrentEntry"
            >
              删除当前角色
            </button>
          </div>

          <div class="editor-layout">
            <section class="editor-main">
              <label class="field">
                <span>角色主名称 <em>先填你真正会拿来搜的名字</em></span>
                <input
                  v-model="form.name"
                  data-field="name"
                  type="text"
                  placeholder="如：阿布 / 老板"
                >
              </label>

              <label class="field">
                <span>别名 <em>昵称、常见写法或外文名；支持逗号或换行分隔</em></span>
                <textarea
                  v-model="form.aliases"
                  data-field="aliases"
                  rows="3"
                  placeholder="如：布布，Abu，阿布老师"
                />
              </label>

              <div class="field">
                <span>示例图 <em>优先放最像这个角色的几张，宁少勿杂</em></span>
                <div class="example-grid">
                  <article
                    v-for="(image, index) in form.exampleImages"
                    :key="image"
                    class="example-card"
                    data-role="example-image-card"
                  >
                    <img
                      class="example-card-image"
                      :src="resolveExampleImageSrc(image)"
                      :alt="`${form.name || '角色'}示例图 ${index + 1}`"
                    >
                    <div class="example-card-overlay">
                      <span class="example-card-title">示例图 {{ index + 1 }}</span>
                      <button
                        class="example-card-remove"
                        data-action="remove-example-image"
                        type="button"
                        @click="removeExampleImage(image)"
                      >
                        移除
                      </button>
                    </div>
                  </article>

                  <button
                    class="example-card import-card"
                    data-role="import-example-card"
                    data-action="import-example-image"
                    type="button"
                    :disabled="importingExample || !selectedEntry"
                    @click="importExampleImage"
                  >
                    <span class="import-card-icon">{{ importingExample ? "…" : "+" }}</span>
                    <span class="import-card-title">{{ importingExample ? "导入中" : "导入示例图" }}</span>
                    <span class="import-card-copy">选择本地图片后自动复制到角色库目录</span>
                  </button>
                </div>
                <span class="mini-note">补完示例图后，直接在下面试一下是否能命中。</span>
              </div>

              <section class="report-card action-card">
                <div class="panel-head compact">
                  <div>
                    <h3>先试一下能不能搜到</h3>
                    <p class="panel-copy">输入你真的会搜的名字、别名或记忆线索，先确认这次补强值不值。</p>
                  </div>
                </div>
                <textarea
                  v-model="testText"
                  data-field="test-text"
                  class="test-text"
                  rows="4"
                  placeholder="输入角色名、别名或你记得的动作/表情线索"
                />
                <button
                  class="primary-btn full"
                  data-action="test-match"
                  :disabled="loading"
                  @click="testMatch"
                >
                  测试召回
                </button>

                <div
                  v-if="matchResult.recommendedName"
                  class="match-summary"
                >
                  最终推荐角色：{{ matchResult.recommendedName }}
                </div>
                <div
                  v-else-if="tested"
                  class="report-ok"
                >
                  未命中任何私有角色
                </div>

                <div class="match-list">
                  <div
                    v-for="item in matchResult.matches"
                    :key="`${item.name}-${item.matchedTerm}-${item.matchType}`"
                    class="match-item"
                  >
                    <div class="match-title">
                      {{ item.name }}
                    </div>
                    <div class="match-meta">
                      {{ item.matchType }} · 命中词：{{ item.matchedTerm }} · 分值：{{ item.score }}
                    </div>
                  </div>
                </div>
              </section>

              <section class="report-card">
                <div class="panel-head compact">
                  <div>
                    <h3>系统已自动检查</h3>
                    <p class="panel-copy">你修改后会自动刷新这里的风险提示，不用再多点一步。</p>
                  </div>
                </div>
                <div
                  v-if="report.errors.length === 0 && report.warnings.length === 0"
                  class="report-ok"
                >
                  当前没有发现结构错误或警告
                </div>
                <div
                  v-else
                  class="report-list"
                >
                  <div
                    v-for="error in report.errors"
                    :key="`error-${error}`"
                    class="report-item error"
                  >
                    {{ error }}
                  </div>
                  <div
                    v-for="warning in report.warnings"
                    :key="`warning-${warning}`"
                    class="report-item warning"
                  >
                    {{ warning }}
                  </div>
                </div>
              </section>

              <details class="extra-fields">
                <summary>更多补充（可选）</summary>

                <div class="extra-fields__body">
                  <label class="field">
                    <span>匹配线索 <em>动作、表情、场景等记忆点；想不到可以先留空</em></span>
                    <textarea
                      v-model="form.matchTerms"
                      data-field="match-terms"
                      rows="4"
                      placeholder="如：撇嘴、冷笑、看报表"
                    />
                  </label>
                </div>
              </details>
            </section>
          </div>
        </section>

        <div
          v-else
          class="empty-state large"
        >
          先从左侧选择一个角色，或者新建角色开始编辑。
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { routerKey, type Router } from "vue-router";

type EntryForm = {
  name: string;
  aliases: string;
  matchTerms: string;
  notes: string;
  exampleImages: string[];
};

type KbEntry = {
  id: string;
  name: string;
  category: "meme" | "source" | "person";
  aliases: string[];
  matchTerms: string[];
  notes: string;
  matchMode: "exact" | "contains" | "exact_or_contains";
  priority: number;
  exampleImages: string[];
};

type ValidationReport = {
  errors: string[];
  warnings: string[];
  conflicts: { term: string; canonicals: string[] }[];
};

type MatchResult = {
  matches: Array<{
    name: string;
    category: string;
    matchType: string;
    matchedTerm: string;
    score: number;
    priority: number;
  }>;
  recommendedName: string | null;
};

type KbStateResponse = {
  path: string;
  knowledgeBase: {
    version: number;
    entries: Array<Omit<KbEntry, "id">>;
  };
  validationReport: ValidationReport;
};

const kbPath = ref("");
const loading = ref(false);
const saving = ref(false);
const importingExample = ref(false);
const dirty = ref(false);
const tested = ref(false);
const statusMessage = ref("");
const filterKeyword = ref("");
const selectedEntryId = ref("");
const testText = ref("");
const syncingForm = ref(false);
const report = ref<ValidationReport>({ errors: [], warnings: [], conflicts: [] });
const matchResult = ref<MatchResult>({ matches: [], recommendedName: null });
const version = ref(1);
const entries = ref<KbEntry[]>([]);
const router = inject<Router | null>(routerKey, null);
let validationTimer: ReturnType<typeof setTimeout> | null = null;

const form = reactive<EntryForm>({
  name: "",
  aliases: "",
  matchTerms: "",
  notes: "",
  exampleImages: [],
});

const selectedEntry = computed(() => entries.value.find((entry) => entry.id === selectedEntryId.value) || null);

const filteredEntries = computed(() => {
  const keyword = filterKeyword.value.trim().toLowerCase();
  if (!keyword) {
    return entries.value;
  }
  return entries.value.filter((entry) => {
    const haystack = [
      entry.name,
      entry.aliases.join(" "),
      entry.matchTerms.join(" "),
      entry.notes,
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(keyword);
  });
});

watch(
  () => ({ ...form }),
  () => {
    if (syncingForm.value) return;
    syncFormToEntry();
    scheduleAutoValidation();
  },
  { deep: true }
);

onMounted(() => {
  loadState();
});

onBeforeUnmount(() => {
  if (validationTimer) {
    clearTimeout(validationTimer);
    validationTimer = null;
  }
});

async function loadState() {
  loading.value = true;
  statusMessage.value = "";
  try {
    const state = await invoke<KbStateResponse>("kb_get_state");
    kbPath.value = state.path;
    version.value = state.knowledgeBase.version;
    entries.value = state.knowledgeBase.entries.map((entry, index) => hydrateEntry(entry, index));
    report.value = state.validationReport;
    selectedEntryId.value = entries.value[0]?.id ?? "";
    syncEntryToForm();
    dirty.value = false;
    tested.value = false;
    matchResult.value = { matches: [], recommendedName: null };
  } catch (error) {
    statusMessage.value = String(error);
  } finally {
    loading.value = false;
  }
}

function hydrateEntry(entry: Omit<KbEntry, "id">, index: number): KbEntry {
  return {
    ...entry,
    id: crypto.randomUUID?.() ?? `kb-entry-${index}-${Date.now()}`,
  };
}

function createEntry() {
  const entry = hydrateEntry(
    {
      name: "",
      category: "person",
      aliases: [],
      matchTerms: [],
      notes: "",
      matchMode: "contains",
      priority: 0,
      exampleImages: [],
    },
    entries.value.length
  );
  entries.value = [entry, ...entries.value];
  selectedEntryId.value = entry.id;
  syncEntryToForm();
  dirty.value = true;
  statusMessage.value = "已新建空白角色，填写后记得保存。";
  scheduleAutoValidation();
}

function selectEntry(id: string) {
  selectedEntryId.value = id;
  syncEntryToForm();
}

function deleteCurrentEntry() {
  if (!selectedEntry.value) return;
  const currentId = selectedEntry.value.id;
  entries.value = entries.value.filter((entry) => entry.id !== currentId);
  selectedEntryId.value = entries.value[0]?.id ?? "";
  syncEntryToForm();
  dirty.value = true;
  statusMessage.value = "已从当前草稿中删除角色，保存后才会写回文件。";
  if (entries.value.length === 0) {
    report.value = { errors: [], warnings: [], conflicts: [] };
    return;
  }
  scheduleAutoValidation();
}

function syncEntryToForm() {
  if (!selectedEntry.value) {
    syncingForm.value = true;
    form.name = "";
    form.aliases = "";
    form.matchTerms = "";
    form.notes = "";
    form.exampleImages = [];
    syncingForm.value = false;
    return;
  }

  syncingForm.value = true;
  form.name = selectedEntry.value.name;
  form.aliases = selectedEntry.value.aliases.join(", ");
  form.matchTerms = selectedEntry.value.matchTerms.join(", ");
  form.notes = selectedEntry.value.notes;
  form.exampleImages = [...selectedEntry.value.exampleImages];
  syncingForm.value = false;
}

function syncFormToEntry() {
  if (!selectedEntry.value) return;
  selectedEntry.value.name = form.name;
  selectedEntry.value.category = "person";
  selectedEntry.value.aliases = parseList(form.aliases);
  selectedEntry.value.matchTerms = parseList(form.matchTerms);
  selectedEntry.value.notes = form.notes.trim();
  selectedEntry.value.matchMode = selectedEntry.value.matchMode || "contains";
  selectedEntry.value.priority = selectedEntry.value.priority || 0;
  selectedEntry.value.exampleImages = [...form.exampleImages];
  dirty.value = true;
}

function parseList(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function buildPayload() {
  syncFormToEntry();
  return {
    version: version.value,
    entries: entries.value.map((entry) => {
      const nextEntry = Object.fromEntries(
        Object.entries(entry).filter(([key]) => key !== "id")
      );
      return nextEntry;
    }),
  };
}

async function importExampleImage() {
  if (!selectedEntry.value) return;
  const selected = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }],
  });
  if (!selected || Array.isArray(selected)) return;

  importingExample.value = true;
  statusMessage.value = "";
  try {
    const relativePath = await invoke<string>("kb_import_example_image", {
      sourcePath: selected,
      name: selectedEntry.value.name || "entry",
    });
    const nextImages = [...form.exampleImages];
    if (!nextImages.includes(relativePath)) {
      nextImages.push(relativePath);
      form.exampleImages = nextImages;
    }
    statusMessage.value = `已导入示例图：${relativePath}`;
  } catch (error) {
    statusMessage.value = String(error);
  } finally {
    importingExample.value = false;
  }
}

async function validateKnowledgeBase() {
  try {
    const nextReport = await invoke<ValidationReport | undefined>("kb_validate_entries", {
      knowledgeBase: buildPayload(),
    });
    if (nextReport) {
      report.value = nextReport;
    }
  } catch (error) {
    statusMessage.value = String(error);
  }
}

function scheduleAutoValidation() {
  if (!selectedEntry.value) return;
  if (validationTimer) {
    clearTimeout(validationTimer);
  }
  validationTimer = setTimeout(() => {
    validationTimer = null;
    void validateKnowledgeBase();
  }, 400);
}

async function saveKnowledgeBase() {
  saving.value = true;
  statusMessage.value = "";
  try {
    const state = await invoke<KbStateResponse>("kb_save_entries", {
      knowledgeBase: buildPayload(),
    });
    kbPath.value = state.path;
    version.value = state.knowledgeBase.version;
    entries.value = state.knowledgeBase.entries.map((entry, index) => hydrateEntry(entry, index));
    report.value = state.validationReport;
    if (entries.value.length > 0) {
      const nextSelected = entries.value.find((entry) => entry.name === form.name);
      selectedEntryId.value = nextSelected?.id ?? entries.value[0].id;
    } else {
      selectedEntryId.value = "";
    }
    syncEntryToForm();
    dirty.value = false;
    statusMessage.value = `已保存到 ${state.path}`;
  } catch (error) {
    statusMessage.value = String(error);
  } finally {
    saving.value = false;
  }
}

async function testMatch() {
  tested.value = true;
  statusMessage.value = "";
  try {
    matchResult.value = await invoke<MatchResult>("kb_test_match_entries", {
      knowledgeBase: buildPayload(),
      text: testText.value,
    });
  } catch (error) {
    statusMessage.value = String(error);
  }
}

function removeExampleImage(target: string) {
  form.exampleImages = form.exampleImages.filter((image) => image !== target);
}

function resolveExampleImageSrc(path: string) {
  const normalizedPath = path.replace(/\\/g, "/");
  if (/^[a-zA-Z]:\//.test(normalizedPath) || normalizedPath.startsWith("/")) {
    return convertFileSrc(normalizedPath);
  }

  const normalizedKbPath = kbPath.value.replace(/\\/g, "/");
  const lastSlashIndex = normalizedKbPath.lastIndexOf("/");
  const baseDir = lastSlashIndex >= 0 ? normalizedKbPath.slice(0, lastSlashIndex) : "";
  const absolutePath = baseDir ? `${baseDir}/${normalizedPath}` : normalizedPath;
  return convertFileSrc(absolutePath);
}

function goBack() {
  if (window.history.length > 1) {
    router?.back();
    return;
  }

  void router?.push("/library");
}
</script>

<style scoped>
.kb-view {
  min-height: 100%;
  padding: 0.9rem;
  box-sizing: border-box;
  background:
    radial-gradient(circle at top right, rgba(229, 126, 63, 0.16), transparent 28%),
    radial-gradient(circle at bottom left, rgba(20, 101, 192, 0.12), transparent 32%),
    linear-gradient(180deg, #fcfaf6 0%, #f3efe7 100%);
  color: #2a221c;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.topbar {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: start;
  gap: 0.75rem;
  padding: 0.95rem 1rem;
  border-radius: 16px;
  background: rgba(255, 252, 245, 0.9);
  border: 1px solid rgba(104, 76, 48, 0.12);
  box-shadow: 0 12px 30px rgba(97, 75, 48, 0.06);
}

.topbar-copy {
  min-width: 0;
}

.eyebrow {
  font-size: 0.72rem;
  letter-spacing: 0.12em;
  color: #8c6b4b;
  margin-bottom: 0.18rem;
}

.topbar h1 {
  font-size: 1.02rem;
  line-height: 1.4;
}

.topbar-actions {
  display: flex;
  gap: 0.5rem;
}

.meta-row {
  display: flex;
  gap: 0.55rem;
  flex-wrap: wrap;
}

.meta-pill {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(104, 76, 48, 0.12);
  padding: 0.28rem 0.68rem;
  font-size: 0.8rem;
}

.meta-pill.dirty {
  color: #ad4b1f;
  border-color: rgba(173, 75, 31, 0.24);
}

.status-line {
  padding: 0.7rem 0.9rem;
  border-radius: 12px;
  background: rgba(255, 247, 228, 0.96);
  border: 1px solid rgba(212, 162, 78, 0.25);
  color: #835d25;
}

.workspace {
  display: grid;
  grid-template-columns: 188px minmax(0, 1fr);
  gap: 0.75rem;
  align-items: start;
}

.entry-rail,
.entry-panel,
.content-stack,
.editor-shell,
.editor-main,
.report-card {
  min-width: 0;
}

.entry-rail {
  position: sticky;
  top: 0;
  align-self: start;
}

.entry-panel,
.editor-shell,
.report-card {
  background: rgba(255, 252, 247, 0.94);
  border: 1px solid rgba(104, 76, 48, 0.12);
  border-radius: 16px;
  box-shadow: 0 12px 30px rgba(97, 75, 48, 0.08);
}

.entry-panel {
  padding: 0.85rem;
}

.content-stack {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.editor-shell {
  padding: 0.9rem;
}

.editor-head,
.panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.6rem;
}

.editor-head {
  margin-bottom: 0.8rem;
}

.panel-head.compact {
  margin-bottom: 0.65rem;
}

.panel-head h3,
.editor-head h2,
.entry-panel h2 {
  font-size: 0.98rem;
}

.panel-copy {
  margin-top: 0.15rem;
  font-size: 0.8rem;
  line-height: 1.45;
  color: #7f6a58;
}

.editor-main {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.action-card {
  border-color: rgba(208, 111, 58, 0.18);
  box-shadow: 0 14px 28px rgba(214, 106, 34, 0.08);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.field span {
  font-size: 0.8rem;
  color: #705c4e;
  display: flex;
  align-items: baseline;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.field em {
  font-style: normal;
  font-size: 0.72rem;
  color: #9a846f;
}

.filter-input,
.field input,
.field textarea,
.test-text {
  width: 100%;
  border: 1px solid #d8cabc;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.95);
  padding: 0.72rem 0.82rem;
  font: inherit;
  color: inherit;
}

.filter-input:focus,
.field input:focus,
.field textarea:focus,
.test-text:focus {
  outline: none;
  border-color: #d06f3a;
  box-shadow: 0 0 0 3px rgba(208, 111, 58, 0.12);
}

.entry-list {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  max-height: calc(100vh - 210px);
  overflow: auto;
  margin-top: 0.65rem;
  padding-right: 0.15rem;
}

.entry-item {
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  border-radius: 12px;
  background: #fff;
  padding: 0.72rem 0.82rem;
  cursor: pointer;
  transition: transform 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
}

.entry-item:hover {
  transform: translateY(-1px);
  border-color: rgba(208, 111, 58, 0.28);
  box-shadow: 0 8px 18px rgba(97, 75, 48, 0.08);
}

.entry-item.active {
  border-color: rgba(208, 111, 58, 0.34);
  background: linear-gradient(135deg, #fff6ef 0%, #fffdf9 100%);
}

.entry-title {
  display: block;
  font-weight: 700;
  margin-bottom: 0.18rem;
}

.entry-meta,
.match-meta {
  font-size: 0.78rem;
  color: #7d6958;
  line-height: 1.4;
}

.example-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 0.7rem;
}

.example-card {
  position: relative;
  min-height: 150px;
  border: 1px solid rgba(104, 76, 48, 0.12);
  border-radius: 16px;
  overflow: hidden;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(247, 240, 232, 0.96));
  box-shadow: 0 12px 26px rgba(97, 75, 48, 0.08);
}

.example-card-image {
  width: 100%;
  height: 100%;
  min-height: 150px;
  object-fit: cover;
  display: block;
  background: linear-gradient(135deg, #f3ece2 0%, #e8ddcf 100%);
}

.example-card-overlay {
  position: absolute;
  inset: auto 0 0 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.55rem;
  padding: 0.6rem;
  background: linear-gradient(180deg, rgba(32, 25, 20, 0) 0%, rgba(32, 25, 20, 0.78) 100%);
}

.example-card-title {
  font-size: 0.76rem;
  font-weight: 600;
  color: #fff7f0;
}

.example-card-remove {
  border: none;
  border-radius: 999px;
  padding: 0.34rem 0.62rem;
  font: inherit;
  font-size: 0.74rem;
  color: #fff7f0;
  background: rgba(255, 255, 255, 0.16);
  cursor: pointer;
}

.example-card-remove:hover {
  background: rgba(255, 255, 255, 0.24);
}

.import-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 0.35rem;
  padding: 0.85rem;
  text-align: left;
  cursor: pointer;
  transition: transform 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
}

.import-card:hover {
  transform: translateY(-1px);
  border-color: rgba(208, 111, 58, 0.24);
  box-shadow: 0 16px 28px rgba(97, 75, 48, 0.1);
}

.import-card:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.import-card-icon {
  width: 2.2rem;
  height: 2.2rem;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  color: #c4541d;
  background: rgba(196, 84, 29, 0.12);
}

.import-card-title {
  font-weight: 700;
  color: #4f3d30;
}

.import-card-copy {
  font-size: 0.76rem;
  line-height: 1.4;
  color: #806b59;
}

.extra-fields {
  border: 1px solid rgba(104, 76, 48, 0.12);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.62);
}

.extra-fields summary {
  cursor: pointer;
  padding: 0.75rem 0.85rem;
  font-weight: 600;
  color: #5f4a39;
  list-style: none;
}

.extra-fields summary::-webkit-details-marker {
  display: none;
}

.extra-fields summary::after {
  content: "展开";
  float: right;
  font-size: 0.76rem;
  font-weight: 400;
  color: #8a7462;
}

.extra-fields[open] summary::after {
  content: "收起";
}

.extra-fields__body {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 0 0.85rem 0.85rem;
}

.report-card {
  padding: 0.9rem;
}

.report-list,
.match-list {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.report-item,
.match-item,
.report-ok,
.empty-state {
  border-radius: 12px;
  padding: 0.68rem 0.8rem;
  background: rgba(255, 255, 255, 0.84);
  border: 1px dashed rgba(104, 76, 48, 0.18);
}

.empty-state.large {
  min-height: 220px;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  background: rgba(255, 252, 247, 0.94);
  border: 1px solid rgba(104, 76, 48, 0.12);
  border-radius: 16px;
}

.report-item.error {
  border-style: solid;
  border-color: rgba(199, 79, 57, 0.2);
  background: rgba(255, 243, 241, 0.92);
  color: #a13c24;
}

.report-item.warning {
  border-style: solid;
  border-color: rgba(212, 140, 62, 0.2);
  background: rgba(255, 245, 232, 0.88);
  color: #9a6027;
}

.match-title {
  font-weight: 700;
  margin-bottom: 0.14rem;
}

.match-summary {
  margin-top: 0.7rem;
  margin-bottom: 0.7rem;
  font-weight: 700;
  color: #b75920;
}

.mini-note {
  font-size: 0.74rem;
  color: #8a7462;
}

.primary-btn,
.ghost-btn,
.danger-btn {
  border: none;
  border-radius: 999px;
  padding: 0.62rem 0.98rem;
  font: inherit;
  cursor: pointer;
  transition: transform 0.18s ease, box-shadow 0.18s ease, opacity 0.18s ease;
}

.primary-btn {
  background: linear-gradient(135deg, #d66a22 0%, #e89233 100%);
  color: #fffaf5;
  box-shadow: 0 12px 24px rgba(214, 106, 34, 0.22);
}

.ghost-btn {
  background: rgba(255, 255, 255, 0.92);
  color: #6b5544;
  border: 1px solid rgba(104, 76, 48, 0.12);
}

.danger-btn {
  background: rgba(255, 238, 234, 0.9);
  color: #c45a45;
  border: 1px solid rgba(196, 90, 69, 0.18);
}

.primary-btn:hover,
.ghost-btn:hover,
.danger-btn:hover {
  transform: translateY(-1px);
}

.primary-btn:disabled,
.ghost-btn:disabled,
.danger-btn:disabled {
  opacity: 0.65;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

.small {
  padding: 0.5rem 0.84rem;
  font-size: 0.8rem;
}

.full {
  width: 100%;
  justify-content: center;
  margin-top: 0.65rem;
}

@media (max-width: 1080px) {
  .workspace,
  .topbar {
    grid-template-columns: 1fr;
  }

  .entry-rail {
    position: static;
  }

  .topbar-actions {
    justify-content: flex-start;
  }
}

@media (max-width: 720px) {
  .kb-view {
    padding: 0.7rem;
  }

  .topbar,
  .entry-panel,
  .editor-shell,
  .report-card {
    padding: 0.8rem;
  }

  .topbar h1 {
    font-size: 0.94rem;
  }

  .example-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
