export type TagCategory = "meme" | "person" | "source" | "custom";
export type TagSourceStrategy =
  | "manual"
  | "ocr"
  | "file_name"
  | "ocr+file_name"
  | "clip_text"
  | "example_image"
  | "fallback";

export interface StructuredTag {
  text: string;
  category: TagCategory;
  isAuto: boolean;
  sourceStrategy: TagSourceStrategy;
  confidence: number;
}

export function createManualTag(text: string): StructuredTag {
  return {
    text,
    category: "custom",
    isAuto: false,
    sourceStrategy: "manual",
    confidence: 1,
  };
}
