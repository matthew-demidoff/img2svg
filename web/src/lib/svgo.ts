import { optimize, type Config } from "svgo/browser";

// preset-default minus the id-affecting passes: layer groups carry stable ids
// that the rest of the app (LayerPreview, stats) references, so they must
// survive optimization untouched.
const config: Config = {
  multipass: true,
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
    "mergePaths",
    "collapseGroups",
    "removeMetadata",
  ],
};

export function optimizeSvg(svg: string): string {
  try {
    const result = optimize(svg, config);
    return result.data;
  } catch {
    // A failed optimization should never break the trace; the unoptimized SVG
    // is still valid output.
    return svg;
  }
}
