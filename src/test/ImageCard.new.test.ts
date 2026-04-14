import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import ImageCard from "@/components/ImageCard.vue";
import type { SearchResult } from "@/stores/search";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => `asset://${path}`,
}));
vi.mock("@/composables/useClipboard", () => ({
  useClipboard: () => ({ copyImage: vi.fn() }),
}));

const base: SearchResult = {
  id: "img-1",
  filePath: "/img.jpg",
  thumbnailPath: "/thumb.jpg",
  fileFormat: "jpg",
  score: 0.8,
  tags: [],
  debugInfo: null,
};

describe("ImageCard — 格式角标", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("GIF 格式显示 GIF 角标", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...base, fileFormat: "gif" }, showDebugInfo: false },
    });
    expect(wrapper.find(".format-badge").exists()).toBe(true);
    expect(wrapper.find(".format-badge").text()).toBe("GIF");
  });

  it("WEBP 格式显示 WEBP 角标", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...base, fileFormat: "webp" }, showDebugInfo: false },
    });
    expect(wrapper.find(".format-badge").text()).toBe("WEBP");
  });

  it("JPG 格式不显示角标", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...base, fileFormat: "jpg" }, showDebugInfo: false },
    });
    expect(wrapper.find(".format-badge").exists()).toBe(false);
  });

  it("PNG 格式不显示角标", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...base, fileFormat: "png" }, showDebugInfo: false },
    });
    expect(wrapper.find(".format-badge").exists()).toBe(false);
  });

  it("fileFormat 大写时也能正确匹配", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...base, fileFormat: "GIF" }, showDebugInfo: false },
    });
    expect(wrapper.find(".format-badge").text()).toBe("GIF");
  });
});

describe("ImageCard — 文件丢失占位图", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("图片加载正常时不显示占位图", () => {
    const wrapper = mount(ImageCard, {
      props: { image: base, showDebugInfo: false },
    });
    expect(wrapper.find(".img-missing").exists()).toBe(false);
    expect(wrapper.find("img").exists()).toBe(true);
  });

  it("图片 error 事件后显示文件丢失占位图", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: base, showDebugInfo: false },
    });
    await wrapper.find("img").trigger("error");
    expect(wrapper.find(".img-missing").exists()).toBe(true);
    expect(wrapper.find(".img-missing").text()).toContain("加载失败");
    expect(wrapper.find("img").exists()).toBe(false);
  });

  it("fileStatus 为 missing 时直接显示图片不存在占位图和状态标记", () => {
    const wrapper = mount(ImageCard, {
      props: { image: { ...base, fileStatus: "missing" }, showDebugInfo: false },
    });
    expect(wrapper.find(".img-missing").text()).toContain("图片不存在");
    expect(wrapper.find(".img-missing").attributes("title")).toBe("原文件已丢失");
    expect(wrapper.find(".status-badge").text()).toContain("文件已丢失");
    expect(wrapper.find("img").exists()).toBe(false);
  });
});

describe("ImageCard — 双击 open 事件", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("双击触发 open 事件并携带 id", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: base, showDebugInfo: false },
    });
    await wrapper.trigger("dblclick");
    expect(wrapper.emitted("open")).toBeTruthy();
    expect(wrapper.emitted("open")![0]).toEqual(["img-1"]);
  });
});

describe("ImageCard — 右键菜单新增项", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("右键菜单包含「查看详情」", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: base, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(document.body.querySelector(".context-menu")?.textContent).toContain("查看详情");
    wrapper.unmount();
  });

  it("右键菜单包含「在文件夹中显示」", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: base, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    expect(document.body.querySelector(".context-menu")?.textContent).toContain("在文件夹中显示");
    wrapper.unmount();
  });

  it("点击「查看详情」触发 open 事件并关闭菜单", async () => {
    const wrapper = mount(ImageCard, {
      props: { image: base, showDebugInfo: false },
      attachTo: document.body,
    });
    await wrapper.trigger("contextmenu");
    const btn = Array.from(document.body.querySelectorAll(".context-menu button"))
      .find(button => button.textContent?.includes("查看详情")) as HTMLButtonElement;
    btn.click();
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("open")).toBeTruthy();
    expect(document.body.querySelector(".context-menu")).toBeNull();
    wrapper.unmount();
  });
});
