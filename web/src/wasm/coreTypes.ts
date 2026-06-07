// Mirror of the public types exposed by the Rust core (crates/img2svg-core).
// Kept in sync by hand; the wasm boundary exchanges JSON, so these are the
// shapes the worker deserializes.

export type ImageClass = "logo" | "illustration" | "photo";

export type ClassOverride = "auto" | ImageClass;

export type PhotoMode = "posterize" | "gradient";

/** Exact palette size, or "auto" to let detail choose. */
export type ColorCount = "auto" | number;

export interface Options {
  /** 0..1, higher keeps more detail at the cost of larger output. */
  fidelity: number;
  /** Number of colors to quantize to, or "auto" to derive from detail. */
  colorCount: ColorCount;
  /** Force a tracing strategy, or let the core detect the image class. */
  classOverride: ClassOverride;
  /** Quantize only to colors present in the source rather than a derived palette. */
  lockToSourcePalette: boolean;
  /** How photographic regions are rendered when the photo path is taken. */
  photoMode: PhotoMode;
  /** Collapse to a single ink color on a transparent background. */
  blackAndWhite: boolean;
}

export interface LayerInfo {
  /** Element id assigned to this layer's group, preserved through optimization. */
  id: string;
  /** sRGB hex, e.g. "#1a2b3c". */
  color: string;
  /** Paths contributed by this layer. */
  pathCount: number;
}

export interface Stats {
  /** Image class the core decided to trace as. */
  detectedClass: ImageClass;
  /** Total <path> elements in the output. */
  pathCount: number;
  /** Color layers in draw order, back to front. */
  layers: LayerInfo[];
  /** Source dimensions actually traced (after any decode-time downscale). */
  width: number;
  height: number;
}

export interface TraceResult {
  svg: string;
  stats: Stats;
}

export const defaultOptions: Options = {
  fidelity: 0.6,
  colorCount: "auto",
  classOverride: "auto",
  lockToSourcePalette: false,
  photoMode: "posterize",
  blackAndWhite: false,
};
