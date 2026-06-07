import * as Comlink from "comlink";
import { decode } from "../lib/decode";
import { optimizeSvg } from "../lib/svgo";
import { trace } from "../wasm/loadCore";
import type { Options, TraceResult } from "../wasm/coreTypes";

async function run(file: File, opts: Options): Promise<TraceResult> {
  const { rgba, width, height } = await decode(file);
  const result = await trace(rgba, width, height, opts);
  return { svg: optimizeSvg(result.svg), stats: result.stats };
}

const PNG_TYPE = "image/png";

async function renderPng(svg: string, width: number, height: number): Promise<Blob> {
  const svgBlob = new Blob([svg], { type: "image/svg+xml" });
  const url = URL.createObjectURL(svgBlob);
  try {
    const bitmap = await createImageBitmap(svgBlob).catch(() => {
      // Some engines won't decode an SVG blob directly; fall back to <img>.
      // OffscreenCanvas workers have no <img>, so the blob path must work.
      throw new Error("Unable to rasterize SVG in worker");
    });
    const canvas = new OffscreenCanvas(width, height);
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      bitmap.close();
      throw new Error("2D canvas context unavailable for PNG render");
    }
    ctx.drawImage(bitmap, 0, 0, width, height);
    bitmap.close();
    return canvas.convertToBlob({ type: PNG_TYPE });
  } finally {
    URL.revokeObjectURL(url);
  }
}

const api = { run, renderPng };

export type TraceWorker = typeof api;

Comlink.expose(api);
