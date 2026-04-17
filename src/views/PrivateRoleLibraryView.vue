<template>
  <div class="kb-view">
    <header class="hero">
      <div>
        <p class="eyebrow">
          Dev Only
        </p>
        <h2>私有角色库维护</h2>
        <p class="hero-copy">
          在同一页里完成角色卡片编辑、结构校验和角色召回测试，保存前就能看到潜在冲突。
        </p>
      </div>
      <div class="hero-actions">
        <button
          class="ghost-btn"
          :disabled="loading"
          @click="loadState"
        >
          重新加载
        </button>
        <button
          class="ghost-btn"
          data-action="validate-kb"
          :disabled="loading"
          @click="validateKnowledgeBase"
        >
          校验
        </button>
        <button
          class="primary-btn"
          data-action="save-kb"
          :disabled="loading || saving"
          @click="saveKnowledgeBase"
        >
          {{ saving ? "保存中..." : "保存角色库" }}
        </button>
      </div>
    </header>

    <div class="meta-row">
      <span class="meta-pill">文件：{{ kbPath || "app_data/knowledge_base.json" }}</span>
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
      <aside class="entry-panel">
        <div class="panel-head">
          <h3>条目列表</h3>
          <button
            class="ghost-btn small"
            data-action="new-entry"
            @click="createEntry"
          >
            新建角色
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
      </aside>

      <section class="editor-panel">
        <div class="panel-head">
          <h3>角色编辑</h3>
          <button
            class="danger-btn small"
            data-action="delete-entry"
            :disabled="!selectedEntry"
            @click="deleteCurrentEntry"
          >
            删除当前角色
          </button>
        </div>

        <div
          v-if="selectedEntry"
          class="form-grid"
        >
          <label class="field wide">
            <span>角色主名称 <em>新 schema 主字段为 name</em></span>
            <input
              v-model="form.name"
              data-field="name"
              type="text"
              placeholder="如：阿布 / 老板"
            >
          </label>

          <label class="field wide">
            <span>别名 <em>角色别称、昵称或常见写法；支持逗号或换行分隔</em></span>
            <textarea
              v-model="form.aliases"
              data-field="aliases"
              rows="3"
              placeholder="用逗号或换行分隔"
            />
          </label>

          <label class="field wide">
            <span>匹配线索 <em>动作、表情、场景等记忆点；支持逗号或换行分隔</em></span>
            <textarea
              v-model="form.matchTerms"
              data-field="match-terms"
              rows="4"
              placeholder="如：撇嘴、冷笑、看报表"
            />
          </label>

          <label class="field wide">
            <span>备注 <em>给维护人看的说明字段，不参与首期核心匹配</em></span>
            <textarea
              v-model="form.notes"
              data-field="notes"
              rows="4"
              placeholder="记录这个角色的使用场景或补充说明"
            />
          </label>

          <label class="field wide">
            <span>示例图 <em>填写相对角色库文件的本地路径，支持逗号或换行分隔</em></span>
            <textarea
              v-model="form.exampleImages"
              data-field="example-images"
              rows="4"
              placeholder="如：kb_examples/abu/sample-1.jpg"
            />
            <div class="example-actions">
              <button
                class="ghost-btn small"
                data-action="import-example-image"
                type="button"
                :disabled="importingExample || !selectedEntry"
                @click="importExampleImage"
              >
                {{ importingExample ? "导入中..." : "导入示例图到应用目录" }}
              </button>
              <span class="mini-note">选择本地图片后会复制到角色库目录下的 `kb_examples/` 并自动填入相对路径。</span>
            </div>
          </label>
        </div>

        <div
          v-else
          class="empty-state large"
        >
          先从左侧选择一个角色，或者新建角色开始编辑。
        </div>
      </section>

      <aside class="inspector-panel">
        <section class="report-card">
          <div class="panel-head">
            <h3>校验报告</h3>
            <span class="mini-note">基于当前草稿</span>
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

        <section class="report-card">
          <div class="panel-head">
            <h3>角色召回测试</h3>
            <span class="mini-note">快速验证角色名、别名和线索词是否能命中</span>
          </div>
          <textarea
            v-model="testText"
            data-field="test-text"
            class="test-text"
            rows="5"
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
      </aside>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type EntryForm = {
  name: string;
  aliases: string;
  matchTerms: string;
  notes: string;
  exampleImages: string;
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

const form = reactive<EntryForm>({
  name: "",
  aliases: "",
  matchTerms: "",
  notes: "",
  exampleImages: "",
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
  },
  { deep: true }
);

onMounted(() => {
  loadState();
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
}

function syncEntryToForm() {
  if (!selectedEntry.value) {
    syncingForm.value = true;
    form.name = "";
    form.aliases = "";
    form.matchTerms = "";
    form.notes = "";
    form.exampleImages = "";
    syncingForm.value = false;
    return;
  }

  syncingForm.value = true;
  form.name = selectedEntry.value.name;
  form.aliases = selectedEntry.value.aliases.join(", ");
  form.matchTerms = selectedEntry.value.matchTerms.join(", ");
  form.notes = selectedEntry.value.notes;
  form.exampleImages = selectedEntry.value.exampleImages.join(", ");
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
  selectedEntry.value.exampleImages = parseList(form.exampleImages);
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
      const nextEntry = { ...entry };
      delete nextEntry.id;
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
    const nextImages = parseList(form.exampleImages);
    if (!nextImages.includes(relativePath)) {
      nextImages.push(relativePath);
      form.exampleImages = nextImages.join(", ");
    }
    statusMessage.value = `已导入示例图：${relativePath}`;
  } catch (error) {
    statusMessage.value = String(error);
  } finally {
    importingExample.value = false;
  }
}

async function validateKnowledgeBase() {
  statusMessage.value = "";
  try {
    report.value = await invoke<ValidationReport>("kb_validate_entries", {
      knowledgeBase: buildPayload(),
    });
    statusMessage.value = report.value.errors.length === 0
      ? "校验完成，可以继续保存或测试。"
      : "校验发现错误，请先修复。";
  } catch (error) {
    statusMessage.value = String(error);
  }
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
</script>

<style scoped>
.kb-view {
  min-height: calc(100vh - 60px);
  padding: 1.25rem;
  background:
    radial-gradient(circle at top right, rgba(229, 126, 63, 0.16), transparent 28%),
    radial-gradient(circle at bottom left, rgba(20, 101, 192, 0.12), transparent 32%),
    linear-gradient(180deg, #fcfaf6 0%, #f3efe7 100%);
  color: #2a221c;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.hero {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding: 1.25rem;
  border-radius: 18px;
  background: rgba(255, 252, 245, 0.92);
  border: 1px solid rgba(104, 76, 48, 0.12);
  box-shadow: 0 14px 40px rgba(97, 75, 48, 0.08);
}

.eyebrow {
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  color: #8c6b4b;
  margin-bottom: 0.4rem;
}

.hero h2 {
  font-size: 2rem;
  margin-bottom: 0.5rem;
}

.hero-copy {
  max-width: 680px;
  line-height: 1.6;
  color: #695748;
}

.hero-actions {
  display: flex;
  gap: 0.75rem;
  align-items: flex-start;
  flex-wrap: wrap;
}

.meta-row {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.meta-pill {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(104, 76, 48, 0.12);
  padding: 0.35rem 0.8rem;
  font-size: 0.85rem;
}

.meta-pill.dirty {
  color: #ad4b1f;
  border-color: rgba(173, 75, 31, 0.24);
}

.status-line {
  padding: 0.85rem 1rem;
  border-radius: 12px;
  background: rgba(255, 247, 228, 0.96);
  border: 1px solid rgba(212, 162, 78, 0.25);
  color: #835d25;
}

.example-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
  margin-top: 0.65rem;
}

.workspace {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr) 360px;
  gap: 1rem;
}

.entry-panel,
.editor-panel,
.inspector-panel {
  min-width: 0;
}

.entry-panel,
.editor-panel,
.report-card {
  background: rgba(255, 252, 247, 0.94);
  border: 1px solid rgba(104, 76, 48, 0.12);
  border-radius: 18px;
  box-shadow: 0 12px 30px rgba(97, 75, 48, 0.08);
}

.entry-panel,
.editor-panel {
  padding: 1rem;
}

.inspector-panel {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.85rem;
}

.panel-head h3 {
  font-size: 1rem;
}

.mini-note {
  font-size: 0.78rem;
  color: #8a7462;
}

.filter-input,
.field input,
.field select,
.field textarea,
.test-text {
  width: 100%;
  border: 1px solid #d8cabc;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.95);
  padding: 0.75rem 0.85rem;
  font: inherit;
  color: inherit;
}

.filter-input:focus,
.field input:focus,
.field select:focus,
.field textarea:focus,
.test-text:focus {
  outline: none;
  border-color: #d06f3a;
  box-shadow: 0 0 0 3px rgba(208, 111, 58, 0.12);
}

.entry-list {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  max-height: 62vh;
  overflow: auto;
  padding-right: 0.15rem;
}

.entry-item {
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  border-radius: 14px;
  background: #fff;
  padding: 0.85rem 0.9rem;
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
  margin-bottom: 0.25rem;
}

.entry-meta,
.match-meta {
  font-size: 0.82rem;
  color: #7d6958;
  line-height: 1.5;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.9rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.field span {
  font-size: 0.85rem;
  color: #705c4e;
  display: flex;
  align-items: baseline;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.field em {
  font-style: normal;
  font-size: 0.76rem;
  color: #9a846f;
}

.field.wide {
  grid-column: 1 / -1;
}

.report-card {
  padding: 1rem;
}

.report-list,
.match-list {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}

.report-item,
.match-item,
.report-ok,
.empty-state {
  border-radius: 14px;
  padding: 0.8rem 0.9rem;
  background: rgba(255, 255, 255, 0.84);
  border: 1px dashed rgba(104, 76, 48, 0.18);
  color: #6d594b;
}

.report-item.error {
  border-style: solid;
  border-color: rgba(191, 65, 65, 0.22);
  color: #a33838;
  background: rgba(255, 242, 242, 0.96);
}

.report-item.warning {
  border-style: solid;
  border-color: rgba(208, 111, 58, 0.22);
  color: #9f5b2c;
  background: rgba(255, 247, 236, 0.98);
}

.match-summary {
  margin-top: 0.8rem;
  margin-bottom: 0.8rem;
  padding: 0.85rem 0.95rem;
  border-radius: 14px;
  background: linear-gradient(135deg, #223a54 0%, #2d5d7f 100%);
  color: #f8fbff;
  font-weight: 600;
}

.match-title {
  font-weight: 700;
  margin-bottom: 0.25rem;
}

.primary-btn,
.ghost-btn,
.danger-btn {
  border: none;
  border-radius: 999px;
  padding: 0.7rem 1.05rem;
  font: inherit;
  cursor: pointer;
  transition: transform 0.18s ease, opacity 0.18s ease, box-shadow 0.18s ease;
}

.primary-btn {
  background: linear-gradient(135deg, #c4541d 0%, #e28e3b 100%);
  color: white;
  box-shadow: 0 10px 24px rgba(196, 84, 29, 0.22);
}

.ghost-btn {
  background: rgba(255, 255, 255, 0.95);
  color: #5a4739;
  border: 1px solid rgba(104, 76, 48, 0.16);
}

.danger-btn {
  background: rgba(163, 56, 56, 0.1);
  color: #a33838;
  border: 1px solid rgba(163, 56, 56, 0.2);
}

.primary-btn:hover,
.ghost-btn:hover,
.danger-btn:hover {
  transform: translateY(-1px);
}

.primary-btn:disabled,
.ghost-btn:disabled,
.danger-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
  transform: none;
}

.small {
  padding: 0.5rem 0.8rem;
  font-size: 0.86rem;
}

.full {
  width: 100%;
  margin-top: 0.75rem;
  margin-bottom: 0.8rem;
}

.large {
  min-height: 220px;
  display: flex;
  align-items: center;
  justify-content: center;
}

@media (max-width: 1180px) {
  .workspace {
    grid-template-columns: 1fr;
  }

  .entry-list {
    max-height: 280px;
  }
}

@media (max-width: 720px) {
  .kb-view {
    padding: 0.9rem;
  }

  .hero {
    flex-direction: column;
  }

  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
