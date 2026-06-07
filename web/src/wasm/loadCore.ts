import type { Options, TraceResult } from "./coreTypes";

// Shape of the wasm-pack generated glue module. Only the parts we call are
// declared; the rest of the module surface is irrelevant here.
interface CoreModule {
  default: (input?: unknown) => Promise<unknown>;
  trace_wasm: (
    rgba: Uint8Array,
    width: number,
    height: number,
    optsJson: string,
  ) => string;
}

const PKG_NOT_BUILT =
  "img2svg core wasm package not found. Build it first with: " +
  "wasm-pack build crates/img2svg-core --target web --out-dir pkg";

let modulePromise: Promise<CoreModule> | null = null;

function loadModule(): Promise<CoreModule> {
  if (!modulePromise) {
    // The path is resolved relative to this file at runtime. The vite-ignore
    // comment stops the bundler from trying to resolve the pkg directory at
    // build time, which lets the web app build before the wasm is generated.
    const url = new URL(
      "../../../crates/img2svg-core/pkg/img2svg_core.js",
      import.meta.url,
    ).href;
    modulePromise = import(/* @vite-ignore */ url).catch((cause: unknown) => {
      modulePromise = null;
      throw new Error(PKG_NOT_BUILT, { cause });
    }) as Promise<CoreModule>;
  }
  return modulePromise;
}

let initPromise: Promise<CoreModule> | null = null;

export function init(): Promise<CoreModule> {
  if (!initPromise) {
    initPromise = loadModule().then(async (mod) => {
      await mod.default();
      return mod;
    });
  }
  return initPromise;
}

export async function trace(
  rgba: Uint8Array,
  width: number,
  height: number,
  opts: Options,
): Promise<TraceResult> {
  const mod = await init();
  const json = mod.trace_wasm(rgba, width, height, JSON.stringify(opts));
  return JSON.parse(json) as TraceResult;
}
