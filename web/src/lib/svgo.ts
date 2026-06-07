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
          // Keep fills as #rrggbb so the layer model and recolor stay reliable.
          convertColors: false,
          // Keep the viewBox so the preview can scale to fit its box.
          removeViewBox: false,
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
