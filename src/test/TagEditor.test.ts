import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import TagEditor from "@/components/TagEditor.vue";

describe("TagEditor", () => {
  it("在对应分组中点击添加时发出带分类的结构化标签数组", async () => {
    const wrapper = mount(TagEditor, {
      props: {
        tags: [],
      },
    });

    await wrapper.findAll(".tag-add-btn")[0].trigger("click");
    await wrapper.get(".tag-inline-input").setValue("新标签");
    await wrapper.get(".tag-inline-input").trigger("keydown.enter");

    const emitted = wrapper.emitted("update:tags");
    expect(emitted).toBeTruthy();
    expect(emitted?.[0]?.[0]).toEqual([
      {
        text: "新标签",
        category: "custom",
        isAuto: false,
        sourceStrategy: "manual",
        confidence: 1,
      },
    ]);
  });
});
