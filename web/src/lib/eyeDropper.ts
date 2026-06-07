// `EyeDropper` ships in Chromium but isn't in the TS DOM lib yet. Declare the
// slice we use and a typed accessor so callers don't reach for `any`.

interface EyeDropperResult {
  sRGBHex: string;
}

interface EyeDropperInstance {
  open: (options?: { signal?: AbortSignal }) => Promise<EyeDropperResult>;
}

type EyeDropperCtor = new () => EyeDropperInstance;

export function getEyeDropper(): EyeDropperCtor | null {
  return (window as unknown as { EyeDropper?: EyeDropperCtor }).EyeDropper ?? null;
}
