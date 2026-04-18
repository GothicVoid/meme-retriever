import { describe, it, expect } from "vitest";
import {
  getRelevanceBadgeClass,
  getRelevanceLabel,
  getRelevanceLevel,
  getUserFacingRelevanceLabel,
} from "@/utils/relevance";

describe("relevance utils", () => {
  it("使用统一阈值划分高相关、较相关、弱相关", () => {
    expect(getRelevanceLevel(0.75)).toBe("high");
    expect(getRelevanceLevel(0.74)).toBe("medium");
    expect(getRelevanceLevel(0.55)).toBe("medium");
    expect(getRelevanceLevel(0.54)).toBe("low");
  });

  it("标签和样式类与阈值划分一致", () => {
    expect(getRelevanceLabel(0.8)).toBe("高相关");
    expect(getRelevanceLabel(0.6)).toBe("较相关");
    expect(getRelevanceLabel(0.4)).toBe("弱相关");

    expect(getUserFacingRelevanceLabel(0.8)).toBe("最像你要找的");
    expect(getUserFacingRelevanceLabel(0.6)).toBe("可能也对");
    expect(getUserFacingRelevanceLabel(0.4)).toBe("不太确定");

    expect(getRelevanceBadgeClass(0.8)).toBe("relevance-badge--strong");
    expect(getRelevanceBadgeClass(0.6)).toBe("relevance-badge--medium");
    expect(getRelevanceBadgeClass(0.4)).toBe("relevance-badge--weak");
  });
});
