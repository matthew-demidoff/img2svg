import { create } from "zustand";
import * as Comlink from "comlink";
import type { Options, TraceResult } from "./wasm/coreTypes";
import { defaultOptions } from "./wasm/coreTypes";
import type { TraceWorker } from "./worker/trace.worker";

export type Status = "idle" | "processing" | "done" | "error";

interface SourceImage {
  file: File;
  /** Object URL for displaying the raw raster in the UI. */
  url: string;
  width: number;
  height: number;
}

interface StoreState {
  source: SourceImage | null;
  options: Options;
  result: TraceResult | null;
  status: Status;
  error: string | null;
  setSource: (file: File) => Promise<void>;
  setOptions: (patch: Partial<Options>) => void;
  trace: () => Promise<void>;
}

let workerProxy: Comlink.Remote<TraceWorker> | null = null;

function worker(): Comlink.Remote<TraceWorker> {
  if (!workerProxy) {
    const instance = new Worker(new URL("./worker/trace.worker.ts", import.meta.url), {
      type: "module",
    });
    workerProxy = Comlink.wrap<TraceWorker>(instance);
  }
  return workerProxy;
}

async function measure(file: File): Promise<{ width: number; height: number }> {
  const bitmap = await createImageBitmap(file);
  const size = { width: bitmap.width, height: bitmap.height };
  bitmap.close();
  return size;
}

export const useStore = create<StoreState>((set, get) => ({
  source: null,
  options: defaultOptions,
  result: null,
  status: "idle",
  error: null,

  async setSource(file) {
    const previous = get().source;
    if (previous) {
      URL.revokeObjectURL(previous.url);
    }
    const { width, height } = await measure(file);
    set({
      source: { file, url: URL.createObjectURL(file), width, height },
      result: null,
      status: "idle",
      error: null,
    });
    await get().trace();
  },

  setOptions(patch) {
    set((state) => ({ options: { ...state.options, ...patch } }));
    if (get().source) {
      void get().trace();
    }
  },

  async trace() {
    const { source, options } = get();
    if (!source) {
      return;
    }
    set({ status: "processing", error: null });
    try {
      const result = await worker().run(source.file, options);
      // Discard the result if the source changed while we were tracing.
      if (get().source?.file !== source.file) {
        return;
      }
      set({ result, status: "done" });
    } catch (err) {
      if (get().source?.file !== source.file) {
        return;
      }
      const message = err instanceof Error ? err.message : String(err);
      set({ status: "error", error: message });
    }
  },
}));

export function getWorker(): Comlink.Remote<TraceWorker> {
  return worker();
}
