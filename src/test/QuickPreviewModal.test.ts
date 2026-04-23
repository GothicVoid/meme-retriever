import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import QuickPreviewModal from "@/components/QuickPreviewModal.vue";
import type { SearchResult } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
}));

const baseImage: SearchResult = {
  id: "img-1",
  filePath: "/img.jpg",
  thumbnailPath: "/thumb.jpg",
  fileFormat: "jpg",
  fileStatus: "normal",
  score: 0.8,
  tags: [],
  debugInfo: null,
};

describe("QuickPreviewModal", () => {
  it("正常图片时显示预览图和完整操作", () => {
    const wrapper = mount(QuickPreviewModal, {
      props: { image: baseImage, canPrev: false, canNext: true },
    });

    expect(wrapper.find(".quick-preview__image").exists()).toBe(true);
    expect(wrapper.text()).toContain("复制");
    expect(wrapper.text()).toContain("查看详情");
    expect(wrapper.text()).toContain("在文件夹中显示");
  });

  it("文件已丢失时显示统一缺图态，并隐藏复制与定位操作", () => {
    const wrapper = mount(QuickPreviewModal, {
      props: {
        image: { ...baseImage, fileStatus: "missing", filePath: "/missing.jpg" },
        canPrev: false,
        canNext: false,
      },
    });

    expect(wrapper.find(".quick-preview__image").exists()).toBe(true);
    expect(wrapper.find(".quick-preview__image").classes()).toContain("quick-preview__image--missing");
    expect(wrapper.find(".quick-preview__missing--overlay").exists()).toBe(true);
    expect(wrapper.text()).toContain("原文件已丢失");
    expect(wrapper.text()).toContain("可查看详情重新定位，或删除这条记录。");
    expect(wrapper.text()).not.toContain("复制");
    expect(wrapper.text()).not.toContain("在文件夹中显示");
    expect(wrapper.text()).toContain("查看详情");
    expect(wrapper.text()).toContain("关闭");
  });

  it("文件已丢失且没有缩略图时退化成纯文字缺图态", () => {
    const wrapper = mount(QuickPreviewModal, {
      props: {
        image: { ...baseImage, fileStatus: "missing", thumbnailPath: "", filePath: "" },
        canPrev: false,
        canNext: false,
      },
    });

    expect(wrapper.find(".quick-preview__image").exists()).toBe(false);
    expect(wrapper.find(".quick-preview__missing--overlay").exists()).toBe(false);
    expect(wrapper.find(".quick-preview__missing").exists()).toBe(true);
  });
});
