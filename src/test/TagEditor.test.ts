import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import TagEditor from "@/components/TagEditor.vue";

describe("TagEditor", () => {
  it("点击添加时发出用户标签数组", async () => {
    const wrapper = mount(TagEditor, {
      props: {
        tags: [],
      },
    });

    expect(wrapper.text()).not.toContain("自定义");
    expect(wrapper.text()).not.toContain("梗");
    expect(wrapper.text()).not.toContain("人物");
    expect(wrapper.text()).not.toContain("出处");

    await wrapper.get(".tag-add-btn").trigger("click");
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
