import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import DetailModal from "@/components/DetailModal.vue";
import { createManualTag } from "@/types/tags";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

const mockInvoke = vi.mocked(invoke);

const mockMeta = {
  id: "img-0",
  filePath: "/img-0.jpg",
  fileName: "img-0.jpg",
  thumbnailPath: "/thumb-0.jpg",
  fileFormat: "jpg",
  width: 800,
  height: 600,
  fileSize: 102400,
  fileStatus: "normal",
  addedAt: 1700000000,
  useCount: 3,
  tags: [],
};

describe("DetailModal pending input", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("直接点保存时会先提交输入框中的待添加标签", async () => {
    mockInvoke
      .mockResolvedValueOnce(mockMeta)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        ...mockMeta,
        tags: [createManualTag("新标签")],
      });

    const wrapper = mount(DetailModal, {
      props: {
        imageId: "img-0",
        images: [
          {
            id: "img-0",
            filePath: "/img-0.jpg",
            thumbnailPath: "/thumb-0.jpg",
            fileFormat: "jpg",
            score: 1,
            tags: [],
            debugInfo: null,
          },
        ],
      },
    });

    await flushPromises();
    await wrapper.findAll(".tag-add-btn")[0].trigger("click");
    await wrapper.get(".tag-inline-input").setValue("新标签");
    await wrapper.get(".save-btn").trigger("click");
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith("update_tags", {
      imageId: "img-0",
      tags: [createManualTag("新标签")],
    });
  });
});
