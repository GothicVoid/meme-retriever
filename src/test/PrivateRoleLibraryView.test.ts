import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import PrivateRoleLibraryView from "@/views/PrivateRoleLibraryView.vue";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);
const mockConvertFileSrc = vi.mocked(convertFileSrc);

const mockState = {
  path: "app_data/knowledge_base.json",
  knowledgeBase: {
    version: 1,
    entries: [
      {
        name: "阿布",
        aliases: ["布布"],
        exampleImages: ["kb_examples/abu/sample-1.jpg"],
      },
      {
        name: "老板",
        aliases: ["王总"],
        exampleImages: ["kb_examples/boss/sample-1.jpg"],
      },
    ],
  },
  validationReport: {
    errors: [],
    warnings: ["检测到潜在冲突词：老板 -> 老板、老板娘"],
    conflicts: [],
  },
};

describe("PrivateRoleLibraryView", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockOpen.mockReset();
    mockConvertFileSrc.mockClear();
  });

  it("挂载时读取私有角色库并展示角色列表", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      return Promise.resolve(undefined);
    });

    const wrapper = mount(PrivateRoleLibraryView);
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("kb_get_state");
    expect(wrapper.text()).toContain("只有角色名搜不到时，再来补图");
    expect(wrapper.text()).toContain("阿布");
    expect(wrapper.text()).toContain("老板");
    expect(wrapper.text()).toContain("检测到潜在冲突词");
    expect(wrapper.find("[data-action='back-to-library']").exists()).toBe(true);
  });

  it("编辑后会自动触发校验并刷新报告", async () => {
    vi.useFakeTimers();
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_validate_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                name: "阿布 Plus",
                exampleImages: ["kb_examples/abu/sample-1.jpg"],
              }),
            ]),
          },
        });
        return Promise.resolve({
          errors: [],
          warnings: ["短词可能较泛，可留意：阿布 Plus -> 阿布"],
          conflicts: [],
        });
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(PrivateRoleLibraryView);
    await flushPromises();

    await wrapper.get("[data-entry='阿布']").trigger("click");
    await wrapper.get("[data-field='name']").setValue("阿布 Plus");
    await vi.advanceTimersByTimeAsync(450);
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith(
      "kb_validate_entries",
      expect.objectContaining({
        knowledgeBase: expect.objectContaining({
          entries: expect.any(Array),
        }),
      })
    );
    expect(wrapper.text()).toContain("“阿布”这个叫法比较短");
    vi.useRealTimers();
  });

  it("点击保存会将当前私有角色库写回", async () => {
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_save_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                name: "阿布",
                aliases: ["布布", "阿布老师"],
                exampleImages: ["kb_examples/abu/sample-1.jpg"],
              }),
            ]),
          },
        });
        return Promise.resolve({
          path: "app_data/knowledge_base.json",
          knowledgeBase: {
            ...mockState.knowledgeBase,
            entries: [
              {
                ...mockState.knowledgeBase.entries[0],
                aliases: ["布布", "阿布老师"],
              },
              mockState.knowledgeBase.entries[1],
            ],
          },
          validationReport: { errors: [], warnings: [], conflicts: [] },
        });
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(PrivateRoleLibraryView);
    await flushPromises();

    await wrapper.get("[data-entry='阿布']").trigger("click");
    await wrapper.get("[data-field='aliases']").setValue("布布, 阿布老师");
    await wrapper.get("[data-action='save-kb']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith(
      "kb_save_entries",
      expect.objectContaining({
        knowledgeBase: expect.objectContaining({
          entries: expect.any(Array),
        }),
      })
    );
    expect(wrapper.text()).toContain("已保存。现在回搜索页按你平时会搜的叫法试试");
  });

  it("示例图会以图片卡片展示并随保存一起提交", async () => {
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_import_example_image") {
        expect(payload).toEqual({
          sourcePath: "/tmp/boss-2.jpg",
          name: "老板",
        });
        return Promise.resolve("kb_examples/boss/sample-2.jpg");
      }
      if (cmd === "kb_save_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                name: "老板",
                exampleImages: ["kb_examples/boss/sample-1.jpg", "kb_examples/boss/sample-2.jpg"],
              }),
            ]),
          },
        });
        return Promise.resolve(mockState);
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(PrivateRoleLibraryView);
    await flushPromises();

    await wrapper.get("[data-entry='老板']").trigger("click");
    expect(wrapper.findAll("[data-role='example-image-card']")).toHaveLength(1);
    expect(wrapper.find("[data-role='import-example-card']").exists()).toBe(true);

    await wrapper.get("[data-action='remove-example-image']").trigger("click");
    expect(wrapper.findAll("[data-role='example-image-card']")).toHaveLength(0);

    mockOpen.mockResolvedValueOnce("/tmp/boss-2.jpg");
    await wrapper.get("[data-action='import-example-image']").trigger("click");
    await wrapper.get("[data-action='save-kb']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith(
      "kb_save_entries",
      expect.objectContaining({
        knowledgeBase: expect.objectContaining({
          entries: expect.any(Array),
        }),
      })
    );
  });

  it("支持选择图片并导入到应用目录", async () => {
    mockOpen.mockResolvedValueOnce("/tmp/source.jpg");
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_import_example_image") {
        expect(payload).toEqual({
          sourcePath: "/tmp/source.jpg",
          name: "老板",
        });
        return Promise.resolve("kb_examples/entry/sample.jpg");
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(PrivateRoleLibraryView);
    await flushPromises();

    await wrapper.get("[data-entry='老板']").trigger("click");
    await wrapper.get("[data-action='import-example-image']").trigger("click");
    await flushPromises();

    expect(mockOpen).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("kb_import_example_image", {
      sourcePath: "/tmp/source.jpg",
      name: "老板",
    });
    expect(wrapper.findAll("[data-role='example-image-card']")).toHaveLength(2);
    expect(wrapper.html()).toContain("asset://app_data/kb_examples/entry/sample.jpg");
  });

  it("维护工具不再暴露旧分类和匹配控制字段，新建角色按主体识别字段保存", async () => {
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_import_example_image") {
        expect(payload).toEqual({
          sourcePath: "/tmp/new-role.jpg",
          name: "新角色",
        });
        return Promise.resolve("kb_examples/new-role/sample-1.jpg");
      }
      if (cmd === "kb_save_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                name: "新角色",
                aliases: ["角色别名"],
                exampleImages: ["kb_examples/new-role/sample-1.jpg"],
              }),
            ]),
          },
        });
        return Promise.resolve(mockState);
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(PrivateRoleLibraryView);
    await flushPromises();

    expect(wrapper.find("[data-field='category']").exists()).toBe(false);
    expect(wrapper.find("[data-field='match-mode']").exists()).toBe(false);
    expect(wrapper.find("[data-field='priority']").exists()).toBe(false);
    expect(wrapper.find("[data-field='notes']").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("匹配线索");

    await wrapper.get("[data-action='new-entry']").trigger("click");
    await wrapper.get("[data-field='name']").setValue("新角色");
    await wrapper.get("[data-field='aliases']").setValue("角色别名");
    mockOpen.mockResolvedValueOnce("/tmp/new-role.jpg");
    await wrapper.get("[data-action='import-example-image']").trigger("click");
    await wrapper.get("[data-action='save-kb']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith(
      "kb_save_entries",
      expect.objectContaining({
        knowledgeBase: expect.objectContaining({
          entries: expect.any(Array),
        }),
      })
    );
  });
});
