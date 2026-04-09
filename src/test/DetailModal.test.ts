import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import DetailModal from "@/components/DetailModal.vue";
import type { SearchResult } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@/components/TagEditor.vue", () => ({
  default: {
    name: "TagEditor",
    props: ["tags"],
    emits: ["update:tags"],
    template: `<div class="tag-editor">{{ tags.join(',') }}</div>`,
  },
}));

const mockInvoke = vi.mocked(invoke);

function makeImages(count = 3): SearchResult[] {
  return Array.from({ length: count }, (_, i) => ({
    id: `img-${i}`,
    filePath: `/img-${i}.jpg`,
    thumbnailPath: `/thumb-${i}.jpg`,
    fileFormat: "jpg",
    score: 0.9 - i * 0.1,
    tags: [`tag${i}`],
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
  addedAt: 1700000000,
  useCount: 3,
  tags: ["tag0"],
};

describe("DetailModal — 渲染", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
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
    mockInvoke.mockResolvedValue(mockMeta);
  });

  it("显示保存标签按钮", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    expect(wrapper.find(".save-btn").exists()).toBe(true);
  });

  it("点击保存标签调用 update_tags", async () => {
    const images = makeImages();
    const wrapper = mount(DetailModal, { props: { imageId: "img-0", images } });
    await flushPromises();
    mockInvoke.mockResolvedValue(undefined);
    await wrapper.find(".save-btn").trigger("click");
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith("update_tags", {
      imageId: "img-0",
      tags: expect.any(Array),
    });
  });
});

describe("DetailModal — GIF 大文件保护", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
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
