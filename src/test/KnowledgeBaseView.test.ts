import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import type { InvokeArgs } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import KnowledgeBaseView from "@/views/KnowledgeBaseView.vue";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

const mockState = {
  path: "app_data/knowledge_base.json",
  knowledgeBase: {
    version: 1,
    entries: [
      {
        canonical: "蚌埠住了",
        category: "meme",
        aliases: ["绷不住了"],
        matchTerms: ["忍不住笑"],
        description: "表示忍不住笑了",
        matchMode: "contains",
        priority: 100,
        exampleImages: [],
      },
      {
        canonical: "甄嬛传",
        category: "source",
        aliases: ["后宫甄嬛传"],
        matchTerms: ["皇上", "臣妾"],
        description: "电视剧出处标签",
        matchMode: "contains",
        priority: 90,
        exampleImages: ["examples/zhenhuan/sample-1.jpg"],
      },
    ],
  },
  validationReport: {
    errors: [],
    warnings: ["检测到潜在冲突词：皇上 -> 甄嬛传、如懿传"],
    conflicts: [],
  },
};

describe("KnowledgeBaseView", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockOpen.mockReset();
  });

  it("挂载时读取知识库并展示条目列表", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      return Promise.resolve(undefined);
    });

    const wrapper = mount(KnowledgeBaseView);
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("kb_get_state");
    expect(wrapper.text()).toContain("知识库维护");
    expect(wrapper.text()).toContain("蚌埠住了");
    expect(wrapper.text()).toContain("甄嬛传");
    expect(wrapper.text()).toContain("检测到潜在冲突词");
  });

  it("编辑后点击校验会调用后端并刷新报告", async () => {
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_validate_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                canonical: "蚌埠住了 Plus",
                exampleImages: [],
              }),
            ]),
          },
        });
        return Promise.resolve({
          errors: [],
          warnings: ["高歧义短词，请确认是否保留：蚌埠住了 Plus -> 笑死"],
          conflicts: [],
        });
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(KnowledgeBaseView);
    await flushPromises();

    await wrapper.get("[data-entry='蚌埠住了']").trigger("click");
    await wrapper.get("[data-field='canonical']").setValue("蚌埠住了 Plus");
    await wrapper.get("[data-action='validate-kb']").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith(
      "kb_validate_entries",
      expect.objectContaining({
        knowledgeBase: expect.objectContaining({
          entries: expect.any(Array),
        }),
      })
    );
    expect(wrapper.text()).toContain("高歧义短词");
  });

  it("点击保存会将当前知识库写回", async () => {
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_save_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                canonical: "蚌埠住了",
                aliases: ["绷不住了", "蚌住了"],
                exampleImages: [],
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
                aliases: ["绷不住了", "蚌住了"],
              },
              mockState.knowledgeBase.entries[1],
            ],
          },
          validationReport: { errors: [], warnings: [], conflicts: [] },
        });
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(KnowledgeBaseView);
    await flushPromises();

    await wrapper.get("[data-entry='蚌埠住了']").trigger("click");
    await wrapper.get("[data-field='aliases']").setValue("绷不住了, 蚌住了");
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
    expect(wrapper.text()).toContain("已保存到");
  });

  it("输入 OCR 文本后可以看到测试命中结果", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_test_match_entries") {
        return Promise.resolve({
          matches: [
            {
              canonical: "甄嬛传",
              category: "source",
              matchType: "MatchTermSubstring",
              matchedTerm: "皇上",
              score: 355,
              priority: 90,
            },
          ],
          recommendedCanonical: "甄嬛传",
        });
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(KnowledgeBaseView);
    await flushPromises();

    await wrapper.get("[data-field='test-text']").setValue("皇上看到这张图都沉默了");
    await wrapper.get("[data-action='test-match']").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("最终推荐标签：甄嬛传");
    expect(wrapper.text()).toContain("命中词：皇上");
  });

  it("支持编辑示例图字段并随保存一起提交", async () => {
    mockInvoke.mockImplementation((cmd: string, payload?: InvokeArgs) => {
      if (cmd === "kb_get_state") return Promise.resolve(mockState);
      if (cmd === "kb_save_entries") {
        expect(payload).toMatchObject({
          knowledgeBase: {
            entries: expect.arrayContaining([
              expect.objectContaining({
                canonical: "甄嬛传",
                exampleImages: ["examples/zhenhuan/sample-1.jpg", "examples/zhenhuan/sample-2.jpg"],
              }),
            ]),
          },
        });
        return Promise.resolve(mockState);
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(KnowledgeBaseView);
    await flushPromises();

    await wrapper.get("[data-entry='甄嬛传']").trigger("click");
    await wrapper
      .get("[data-field='example-images']")
      .setValue("examples/zhenhuan/sample-1.jpg, examples/zhenhuan/sample-2.jpg");
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
          canonical: "甄嬛传",
        });
        return Promise.resolve("kb_examples/entry/sample.jpg");
      }
      return Promise.resolve(undefined);
    });

    const wrapper = mount(KnowledgeBaseView);
    await flushPromises();

    await wrapper.get("[data-entry='甄嬛传']").trigger("click");
    await wrapper.get("[data-action='import-example-image']").trigger("click");
    await flushPromises();

    expect(mockOpen).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith("kb_import_example_image", {
      sourcePath: "/tmp/source.jpg",
      canonical: "甄嬛传",
    });
    expect((wrapper.get("[data-field='example-images']").element as HTMLTextAreaElement).value)
      .toContain("kb_examples/entry/sample.jpg");
  });
});
