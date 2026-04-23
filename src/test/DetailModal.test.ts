import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import DetailModal from "@/components/DetailModal.vue";
import type { SearchResult } from "@/stores/search";
import { createManualTag, type StructuredTag } from "@/types/tags";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));
vi.mock("@/components/TagEditor.vue", () => ({
  default: {
    name: "TagEditor",
    props: ["tags"],
    emits: ["update:tags"],
    template: `
      <div class="tag-editor">
        {{ tags.map((tag) => tag.text).join(',') }}
        <button
          class="mock-add-tag"
          @click="$emit('update:tags', [...tags, { text: '新增', category: 'custom', isAuto: false, sourceStrategy: 'manual', confidence: 1 }])"
        >
          add
        </button>
      </div>
    `,
  },
}));

const mockInvoke = vi.mocked(invoke);
const mockOpen = vi.mocked(open);

function makeImages(count = 3): SearchResult[] {
  return Array.from({ length: count }, (_, i) => ({
    id: `img-${i}`,
    filePath: `/img-${i}.jpg`,
    thumbnailPath: `/thumb-${i}.jpg`,
    fileFormat: "jpg",
    score: 0.9 - i * 0.1,
    tags: [createManualTag(`tag${i}`)],
    debugInfo: null,
  }));
}

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
  tags: [createManualTag("tag0")],
};

const autoTag: StructuredTag = {
  text: "自动猫",
  category: "meme",
  isAuto: true,
  sourceStrategy: "clip_text",
  confidence: 0.6,
};

describe("DetailModal — 渲染", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockOpen.mockReset();
    mockInvoke.mockResolvedValue(mockMeta);
  });

  it("挂载后调用 get_image_meta", async () => {
    const images = makeImages();
    mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("get_image_meta", { id: "img-0" });
  });

  it("显示图片元素", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".main-img").exists()).toBe(true);
  });

  it("加载元数据后显示尺寸", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.text()).toContain("800");
    expect(wrapper.text()).toContain("600");
  });

  it("加载元数据后显示文件大小（KB）", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.text()).toContain("KB");
  });

  it("点击背景触发 close 事件", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await wrapper.find(".modal-backdrop").trigger("click");
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("点击关闭按钮触发 close 事件", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await wrapper.find(".close-btn").trigger("click");
    expect(wrapper.emitted("close")).toBeTruthy();
  });
});

describe("DetailModal — 键盘导航", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockOpen.mockReset();
    mockInvoke.mockResolvedValue(mockMeta);
  });

  it("第一张图片时不显示上一张按钮", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".prev").exists()).toBe(false);
  });

  it("最后一张图片时不显示下一张按钮", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, { props: { imageId: "img-2", images } });
    await flushPromises();
    expect(wrapper.find(".next").exists()).toBe(false);
  });

  it("中间图片时两个导航按钮都显示", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, { props: { imageId: "img-1", images } });
    await flushPromises();
    expect(wrapper.find(".prev").exists()).toBe(true);
    expect(wrapper.find(".next").exists()).toBe(true);
  });

  it("点击下一张切换图片并重新加载元数据", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    mockInvoke.mockResolvedValue({ ...mockMeta, id: "img-1" });
    await wrapper.find(".next").trigger("click");
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("get_image_meta", { id: "img-1" });
  });

  it("ESC 键触发 close 事件", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, {
      props: { imageId: "img-1", images },
      attachTo: document.body,
    });
    await flushPromises();
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("close")).toBeTruthy();
    wrapper.unmount();
  });

  it("ArrowRight 键切换到下一张", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, {
      props: { imageId: "img-0", images },
      attachTo: document.body,
    });
    await flushPromises();
    mockInvoke.mockResolvedValue({ ...mockMeta, id: "img-1" });
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }));
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("get_image_meta", { id: "img-1" });
    wrapper.unmount();
  });

  it("ArrowLeft 键切换到上一张", async () => {
    const images = makeImages(3);
    const wrapper = mount(DetailModal, {
      props: { imageId: "img-2", images },
      attachTo: document.body,
    });
    await flushPromises();
    mockInvoke.mockResolvedValue({ ...mockMeta, id: "img-1" });
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft" }));
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("get_image_meta", { id: "img-1" });
    wrapper.unmount();
  });
});

describe("DetailModal — 标签保存", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockOpen.mockReset();
    mockInvoke.mockResolvedValue(mockMeta);
  });

  it("显示保存标签按钮", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".save-btn").exists()).toBe(true);
  });

  it("详情页只把用户标签传给标签编辑器", async () => {
    mockInvoke.mockResolvedValueOnce({
      ...mockMeta,
      tags: [createManualTag("tag0"), autoTag],
    });
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();

    expect(wrapper.text()).toContain("标签");
    expect(wrapper.text()).toContain("添加你以后会用来搜索这张图的词");
    expect(wrapper.text()).toContain("tag0");
    expect(wrapper.text()).not.toContain("自动猫");
    expect(wrapper.text()).not.toContain("按分类分组管理");
  });

  it("点击保存标签调用 update_tags，并保留隐藏的内部标签", async () => {
    mockInvoke.mockResolvedValueOnce({
      ...mockMeta,
      tags: [createManualTag("tag0"), autoTag],
    });
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    await wrapper.get(".mock-add-tag").trigger("click");
    mockInvoke.mockResolvedValue(mockMeta);
    await wrapper.find(".save-btn").trigger("click");
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("update_tags", {
      imageId: "img-0",
      tags: [autoTag, createManualTag("tag0"), createManualTag("新增")],
    });
  });
});

describe("DetailModal — GIF 大文件保护", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockOpen.mockReset();
  });

  it("小 GIF（<10MB）不显示播放按钮", async () => {
    mockInvoke.mockResolvedValue({
      ...mockMeta,
      fileFormat: "gif",
      fileSize: 1024 * 1024, // 1MB
    });
    const images = [{ ...makeImages(1)[0], fileFormat: "gif" }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".gif-toggle").exists()).toBe(false);
  });

  it("大 GIF（>10MB）显示播放按钮", async () => {
    mockInvoke.mockResolvedValue({
      ...mockMeta,
      fileFormat: "gif",
      fileSize: 11 * 1024 * 1024, // 11MB
    });
    const images = [{ ...makeImages(1)[0], fileFormat: "gif" }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".gif-toggle").exists()).toBe(true);
    expect(wrapper.find(".gif-toggle").text()).toContain("播放");
  });
});

describe("DetailModal — 文件丢失", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    mockOpen.mockReset();
  });

  it("文件已丢失时显示错误态", async () => {
    mockInvoke.mockResolvedValue({
      ...mockMeta,
      fileStatus: "missing",
    });
    const images = [{ ...makeImages(1)[0], fileStatus: "missing" }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.text()).toContain("原文件已丢失");
    expect(wrapper.find(".relocate-btn").exists()).toBe(true);
    expect(wrapper.find(".save-btn").exists()).toBe(false);
    expect(wrapper.find(".tag-editor").exists()).toBe(false);
    expect(wrapper.find(".main-img").exists()).toBe(true);
    expect(wrapper.find(".main-img").classes()).toContain("main-img--missing");
    expect(wrapper.find(".missing-state--overlay").exists()).toBe(true);
  });

  it("文件已丢失时展示重新定位所需的识别线索", async () => {
    mockInvoke.mockResolvedValue({
      ...mockMeta,
      fileStatus: "missing",
      filePath: "/archive/memes/img-0.jpg",
      fileName: "img-0.jpg",
      tags: [createManualTag("tag0"), createManualTag("reaction"), autoTag],
    });
    const images = [{
      ...makeImages(1)[0],
      fileStatus: "missing",
      filePath: "/archive/memes/img-0.jpg",
      tags: [createManualTag("tag0"), createManualTag("reaction"), autoTag],
    }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();

    expect(wrapper.text()).toContain("识别线索");
    expect(wrapper.text()).toContain("文件名");
    expect(wrapper.text()).toContain("img-0.jpg");
    expect(wrapper.text()).toContain("原路径");
    expect(wrapper.text()).toContain("/archive/memes/img-0.jpg");
    expect(wrapper.text()).toContain("已有标签");
    expect(wrapper.text()).toContain("reaction");
    expect(wrapper.text()).not.toContain("自动猫");
  });

  it("文件已丢失时显示删除按钮并触发 delete 事件", async () => {
    mockInvoke.mockResolvedValue({
      ...mockMeta,
      fileStatus: "missing",
    });
    const images = [{ ...makeImages(1)[0], fileStatus: "missing" }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".delete-btn").exists()).toBe(true);
    await wrapper.find(".delete-btn").trigger("click");
    expect(wrapper.emitted("delete")).toEqual([["img-0"]]);
  });

  it("点击重新定位后调用 relocate_image", async () => {
    mockInvoke
      .mockResolvedValueOnce({
        ...mockMeta,
        fileStatus: "missing",
      })
      .mockResolvedValueOnce({
        ...mockMeta,
        fileStatus: "normal",
        filePath: "/new.jpg",
        fileName: "new.jpg",
      });
    mockOpen.mockResolvedValue("/new.jpg");

    const images = [{ ...makeImages(1)[0], fileStatus: "missing" }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    await wrapper.find(".relocate-btn").trigger("click");
    await flushPromises();

    expect(mockOpen).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenLastCalledWith("relocate_image", {
      id: "img-0",
      newPath: "/new.jpg",
    });
  });

  it("文件已丢失且没有缩略图时显示纯文字失效态", async () => {
    mockInvoke.mockResolvedValue({
      ...mockMeta,
      fileStatus: "missing",
      thumbnailPath: "",
    });
    const images = [{ ...makeImages(1)[0], fileStatus: "missing", thumbnailPath: "" }];
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();

    expect(wrapper.find(".main-img").exists()).toBe(false);
    expect(wrapper.find(".missing-state--overlay").exists()).toBe(false);
    expect(wrapper.find(".missing-state").exists()).toBe(true);
  });
});
