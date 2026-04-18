export type RelevanceLevel = "high" | "medium" | "low";

export const HIGH_RELEVANCE_SCORE = 0.75;
export const MEDIUM_RELEVANCE_SCORE = 0.55;

export function getRelevanceLevel(score: number): RelevanceLevel {
  if (score >= HIGH_RELEVANCE_SCORE) return "high";
  if (score >= MEDIUM_RELEVANCE_SCORE) return "medium";
  return "low";
}

export function getRelevanceLabel(score: number): string {
  const level = getRelevanceLevel(score);
  if (level === "high") return "高相关";
  if (level === "medium") return "较相关";
  return "弱相关";
}

export function getRelevanceBadgeClass(score: number): string {
  const level = getRelevanceLevel(score);
  if (level === "high") return "relevance-badge--strong";
  if (level === "medium") return "relevance-badge--medium";
  return "relevance-badge--weak";
}
