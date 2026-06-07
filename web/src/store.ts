import { create } from "zustand";
import type { Options, TraceResult } from "./wasm/coreTypes";
import { defaultOptions } from "./wasm/coreTypes";
import { runTrace } from "./lib/pipeline";

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
  /** Update options now but coalesce the re-trace; for dragged controls. */
  setOptionsDebounced: (patch: Partial<Options>) => void;
  trace: () => Promise<void>;
}

const TRACE_DEBOUNCE_MS = 220;

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

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
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    set((state) => ({ options: { ...state.options, ...patch } }));
    if (get().source) {
      void get().trace();
    }
  },

  setOptionsDebounced(patch) {
    set((state) => ({ options: { ...state.options, ...patch } }));
    if (!get().source) {
      return;
    }
    set({ status: "processing" });
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
      debounceTimer = null;
      void get().trace();
    }, TRACE_DEBOUNCE_MS);
  },

  async trace() {
    const { source, options } = get();
    if (!source) {
      return;
    }
    set({ status: "processing", error: null });
    try {
      const result = await runTrace(source.file, options);
      // Drop the result if the source changed while we were tracing.
      if (get().source?.file !== source.file) {
        return;
      }
      set({ result, status: "done" });
    } catch (err) {
      if (get().source?.file !== source.file) {
        return;
      }
      set({
        status: "error",
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },
}));
