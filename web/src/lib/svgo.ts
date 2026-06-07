import { optimize, type Config } from "svgo/browser";

// preset-default minus the id-affecting passes: layer groups carry stable ids
// that the rest of the app references, so they must survive untouched.
const config: Config = {
  multipass: false,
  floatPrecision: 2,
  plugins: [
    {
      name: "preset-default",
      params: {
        overrides: {
          cleanupIds: false,
          convertPathData: { floatPrecision: 2 },
          cleanupNumericValues: { floatPrecision: 2 },
        },
      },
    },
  ],
};

export function optimizeSvg(svg: string): string {
  try {
    return optimize(svg, config).data;
  } catch {
    // A failed optimization should never break the trace; the unoptimized SVG
    // is still valid output.
    return svg;
  }
}
