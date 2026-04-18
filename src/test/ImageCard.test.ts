import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import ImageCard from "@/components/ImageCard.vue";
import Toast from "@/components/Toast.vue";
import type { SearchResult } from "@/stores/search";
import { createManualTag } from "@/types/tags";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));

const copyImageMock = vi.fn();
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: copyImageMock }),
}));

const mockImage: SearchResult = {
  id: "uuid-1",
  filePath: "/library/images/uuid-1.jpg",
  thumbnailPath: "/library/thumbs/uuid-1.jpg",
  fileFormat: "jpg",
  score: 0.9,
  tags: [createManualTag("搞笑")],
  debugInfo: null,
};

describe("ImageCard", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    copyImageMock.mockReset();
  });

  it("渲染缩略图", () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    const img = wrapper.find("img");
    expect(img.exists()).toBe(true);
    expect(img.attributes("src")).toContain("uuid-1");
  });

  it("挂载统一结果卡骨架类，便于首页和搜索结果复用同一视觉语言", () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    expect(wrapper.get(".image-card").classes()).toContain("ui-result-card");
    expect(wrapper.get(".image-media").classes()).toContain("ui-result-card__media");
  });

  it("右键点击显示上下文菜单", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(document.body.querySelector(".context-menu")).not.toBeNull();
    wrapper.unmount();
  });

  it("上下文菜单包含删除选项", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(document.body.querySelector(".context-menu")?.textContent).toContain("删除");
    wrapper.unmount();
  });

  it("点击删除菜单项触发 delete 事件", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    const deleteBtn = document.body.querySelector("[data-action='delete']") as HTMLElement;
    deleteBtn.click();
    expect(wrapper.emitted("delete")).toBeTruthy();
    expect(wrapper.emitted("delete")![0]).toEqual(["uuid-1"]);
    wrapper.unmount();
  });

  it("点击其他区域关闭上下文菜单", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(document.body.querySelector(".context-menu")).not.toBeNull();

    await document.dispatchEvent(new MouseEvent("click"));
    await wrapper.vm.$nextTick();
    expect(document.body.querySelector(".context-menu")).toBeNull();

    wrapper.unmount();
  });

  it("打开第二个右键菜单时关闭前一个菜单", async () => {
    const first = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
      attachTo: document.body,
    });
    const second = mount(ImageCard, {
      props: {
        image: { ...mockImage, id: "uuid-2", filePath: "/library/images/uuid-2.jpg", thumbnailPath: "/library/thumbs/uuid-2.jpg" },
        showDebugInfo: false,
      },
      attachTo: document.body,
    });

    await first.trigger("contextmenu", { clientX: 20, clientY: 30 });
    expect(document.body.querySelectorAll(".context-menu")).toHaveLength(1);

    await second.trigger("contextmenu", { clientX: 60, clientY: 70 });
    expect(document.body.querySelectorAll(".context-menu")).toHaveLength(1);

    const menu = document.body.querySelector(".context-menu") as HTMLElement;
    expect(menu.style.left).toBe("60px");
    expect(menu.style.top).toBe("70px");

    first.unmount();
    second.unmount();
  });

  it("showDebugInfo=false 时不显示调试叠层", () => {
    const wrapper = mount(ImageCard, { props: { image: mockImage, showDebugInfo: false } });
    expect(wrapper.find(".debug-overlay").exists()).toBe(false);
  });

  it("有 debugInfo 时默认显示用户态排序理由", () => {
    const image: SearchResult = {
      ...mockImage,
      score: 0.82,
      matchedOcrTerms: ["撤回"],
      matchedTags: ["聊天截图"],
      debugInfo: {
        mainRoute: "ocr",
        mainScore: 0.8,
        auxScore: 0.2,
        semScore: 0.4,
        kwScore: 0.72,
        tagScore: 0.25,
        popularityBoost: 0.02,
      },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: false } });
    const reasonPanel = wrapper.find(".reason-panel");
    expect(reasonPanel.exists()).toBe(true);
    expect(reasonPanel.text()).toContain("最像你要找的");
    expect(reasonPanel.text()).toContain("命中文字");
    expect(reasonPanel.text()).toContain("命中文字：撤回");
    expect(reasonPanel.text()).toContain("标签命中：聊天截图");
    expect(wrapper.find(".image-media").exists()).toBe(true);
    expect(reasonPanel.classes()).toContain("ui-result-card__info");
  });

  it("普通模式下角色主路优先显示角色命中", () => {
    const image: SearchResult = {
      ...mockImage,
      score: 0.77,
      matchedRoleName: "阿布",
      matchedTags: ["撇嘴"],
      debugInfo: {
        mainRoute: "privateRole",
        mainScore: 0.8,
        auxScore: 0.1,
        semScore: 0.52,
        kwScore: 0,
        tagScore: 0.2,
        popularityBoost: 0,
      },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: false } });
    const reasonPanel = wrapper.find(".reason-panel");
    expect(reasonPanel.text()).toContain("最像你要找的");
    expect(reasonPanel.text()).toContain("角色命中");
    expect(reasonPanel.text()).toContain("角色命中：阿布");
    expect(reasonPanel.text()).not.toContain("角色匹配优先");
  });

  it("普通模式下语义主路显示图片内容接近，并限制为主理由加一条补充证据", () => {
    const image: SearchResult = {
      ...mockImage,
      score: 0.61,
      matchedOcrTerms: ["撤回"],
      matchedTags: ["聊天截图"],
      matchedRoleName: "阿布",
      debugInfo: {
        mainRoute: "semantic",
        mainScore: 0.65,
        auxScore: 0.2,
        semScore: 0.7,
        kwScore: 0.4,
        tagScore: 0.3,
        popularityBoost: 0.08,
      },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: false } });
    const reasonPanel = wrapper.find(".reason-panel");
    const pills = wrapper.findAll(".reason-pill");

    expect(reasonPanel.text()).toContain("可能也对");
    expect(reasonPanel.text()).toContain("图片内容接近");
    expect(reasonPanel.text()).not.toContain("语义最接近");
    expect(pills).toHaveLength(2);
    expect(pills[0].text()).toContain("图片内容接近");
  });

  it("showDebugInfo=true 且 debugInfo 为 null 时不显示叠层", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...mockImage, debugInfo: null }, showDebugInfo: true },
    });
    expect(wrapper.find(".debug-overlay").exists()).toBe(false);
  });

  it("showDebugInfo=true 且有 debugInfo 时显示叠层", () => {
    const image: SearchResult = {
      ...mockImage,
      debugInfo: {
        mainRoute: "ocr",
        mainScore: 0.8,
        auxScore: 0.2,
        semScore: 0.8,
        kwScore: 0.3,
        tagScore: 0,
        popularityBoost: 0.05,
      },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: true } });
    const overlay = wrapper.find(".debug-overlay");
    expect(overlay.exists()).toBe(true);
    expect(overlay.text()).toContain("最终得分");
    expect(overlay.text()).toContain("主路 文字");
    expect(overlay.text()).toContain("80");
    expect(overlay.text()).toContain("辅路补充");
    expect(overlay.text()).toContain("标签贡献");
    expect(overlay.text()).toContain("热度加成");
  });

  it("调试层不再显示旧的 CLIP/OCR/标签逐项原始名称", () => {
    const image: SearchResult = {
      ...mockImage,
      debugInfo: {
        mainRoute: "semantic",
        mainScore: 0.6,
        auxScore: 0.2,
        semScore: 0.5,
        kwScore: 0.9,
        tagScore: 0.7,
        popularityBoost: 0.04,
      },
    };
    const wrapper = mount(ImageCard, { props: { image, showDebugInfo: true } });
    const text = wrapper.find(".debug-overlay").text();
    expect(text).toContain("主路 语义");
    expect(text).toContain("60");
    expect(text).not.toContain("CLIP");
    expect(text).not.toContain("OCR");
    expect(text).not.toContain("标签 70%");
  });

  it("selectable=true 时渲染 checkbox", () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false, selectable: true, selected: false },
    });
    expect(wrapper.find("input[type='checkbox']").exists()).toBe(true);
  });

  it("selectable 未传时不渲染 checkbox", () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false },
    });
    expect(wrapper.find("input[type='checkbox']").exists()).toBe(false);
  });

  it("点击 checkbox 触发 select 事件", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: mockImage, showDebugInfo: false, selectable: true, selected: false },
    });
    await wrapper.find("input[type='checkbox']").trigger("change");
    expect(wrapper.emitted("select")).toBeTruthy();
    expect(wrapper.emitted("select")![0]).toEqual(["uuid-1"]);
  });

  it("单击图片时复制到剪贴板并显示成功提示", async () => {
    copyImageMock.mockResolvedValue(undefined);
    const wrapper = mount({
      components: { ImageCard, Toast },
      template: '<div><ImageCard :image="image" :show-debug-info="false" /><Toast /></div>',
      data: () => ({ image: mockImage }),
    }, { attachTo: document.body });

    await wrapper.find(".image-card").trigger("click");
    expect(copyImageMock).toHaveBeenCalledWith("uuid-1");

    const toast = document.body.querySelector(".toast");
    expect(toast?.textContent).toContain("已复制");

    wrapper.unmount();
  });

  it("复制失败时显示失败提示", async () => {
    copyImageMock.mockRejectedValue(new Error("copy failed"));
    const wrapper = mount({
      components: { ImageCard, Toast },
      template: '<div><ImageCard :image="image" :show-debug-info="false" /><Toast /></div>',
      data: () => ({ image: mockImage }),
    }, { attachTo: document.body });

    await wrapper.find(".image-card").trigger("click");

    const toast = document.body.querySelector(".toast.error");
    expect(toast?.textContent).toContain("复制失败");

    wrapper.unmount();
  });
});
