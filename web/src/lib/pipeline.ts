import { decode } from "./decode";
import { optimizeSvg } from "./svgo";
import { traceRaw } from "../wasm/loadCore";
import type { ImageClass, LayerInfo, Options, TraceResult } from "../wasm/coreTypes";

// The UI options are friendlier than the core's. The core uses snake_case
// names, an explicit capitalized class enum, and a separate palette-lock list,
// so translate here to what the Rust side deserializes.
function toCoreOptions(o: Options): Record<string, unknown> {
  const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
  return {
    class_override: o.classOverride === "auto" ? null : capitalize(o.classOverride),
    k: o.colorCount === "auto" ? null : o.colorCount,
    lock_palette: null,
    bw_mode: o.blackAndWhite,
    photo_mode: "Posterize",
    detail: o.fidelity,
  };
}

// VTracer emits one `<path ... fill="#RRGGBB">` per stacked color layer.
function layersFromSvg(svg: string): LayerInfo[] {
  const counts = new Map<string, number>();
  const fill = /fill="(#[0-9a-fA-F]{6})"/g;
  for (let m = fill.exec(svg); m !== null; m = fill.exec(svg)) {
    const color = m[1].toLowerCase();
    counts.set(color, (counts.get(color) ?? 0) + 1);
  }
  return [...counts.entries()].map(([color, pathCount], i) => ({
    id: `layer-${i}`,
    color,
    pathCount,
  }));
}

export async function runTrace(file: File, opts: Options): Promise<TraceResult> {
  const { rgba, width, height } = await decode(file);
  const core = await traceRaw(rgba, width, height, toCoreOptions(opts));
  return {
    svg: optimizeSvg(core.svg),
    stats: {
      detectedClass: core.stats.classified_as.toLowerCase() as ImageClass,
      pathCount: core.stats.path_count,
      layers: layersFromSvg(core.svg),
      width,
      height,
    },
  };
}

export async function renderPng(svg: string, width: number, height: number): Promise<Blob> {
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  try {
    const image = new Image();
    await new Promise<void>((resolve, reject) => {
      image.onload = () => resolve();
      image.onerror = () => reject(new Error("could not rasterize the SVG"));
      image.src = url;
    });
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("2D canvas context unavailable");
    }
    ctx.drawImage(image, 0, 0, width, height);
    return await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new Error("PNG encoding failed"))),
        "image/png",
      );
    });
  } finally {
    URL.revokeObjectURL(url);
  }
}
