import type { ClassOverride, ColorCount } from "../wasm/coreTypes";

export const CLASS_OPTIONS: { value: ClassOverride; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: "logo", label: "Logo" },
  { value: "illustration", label: "Illustration" },
  { value: "photo", label: "Photo" },
];

export const COLOR_OPTIONS: { value: ColorCount; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: 8, label: "8" },
  { value: 16, label: "16" },
  { value: 32, label: "32" },
  { value: 64, label: "64" },
  { value: 128, label: "128" },
  { value: 256, label: "256" },
];

export function parseColorCount(raw: string): ColorCount {
  return raw === "auto" ? "auto" : Number(raw);
}
