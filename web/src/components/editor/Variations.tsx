import { useState } from "react";
import { useStore } from "../../store";
import { runTrace } from "../../lib/pipeline";
import type { ClassOverride, ColorCount, Options } from "../../wasm/coreTypes";

interface VariationSpec {
  label: string;
  patch: Partial<Options>;
}

interface Variation extends VariationSpec {
  svg: string;
  colorCount: number;
  pathCount: number;
}

// A deliberate spread of detail/colorCount, two that vary the class, plus a
// couple of lightly jittered combos for trial-and-error. Jitter is derived
// from the index so runs are reproducible and we never touch Math.random.
function variationSpecs(base: Options): VariationSpec[] {
  const colorChoices: ColorCount[] = [8, 16, 32, 64];
  const classChoices: ClassOverride[] = ["logo", "illustration"];
  const jitterDetail = (i: number) => {
    const delta = ((i % 3) - 1) * 0.1;
    return Math.min(1, Math.max(0, base.fidelity + delta));
  };
  return [
    { label: "Low detail · 8", patch: { fidelity: 0.3, colorCount: colorChoices[0] } },
    { label: "Medium · 16", patch: { fidelity: 0.55, colorCount: colorChoices[1] } },
    { label: "High detail · 32", patch: { fidelity: 0.8, colorCount: colorChoices[2] } },
    { label: "Max · 64", patch: { fidelity: 1, colorCount: colorChoices[3] } },
    { label: "As logo", patch: { classOverride: classChoices[0], colorCount: "auto" } },
    {
      label: "As illustration",
      patch: { classOverride: classChoices[1], fidelity: jitterDetail(4) },
    },
  ];
}

export function Variations() {
  const source = useStore((s) => s.source);
  const options = useStore((s) => s.options);
  const setOptions = useStore((s) => s.setOptions);

  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState(0);
  const [variations, setVariations] = useState<Variation[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function generate() {
    if (!source || running) {
      return;
    }
    setRunning(true);
    setError(null);
    setVariations([]);
    const specs = variationSpecs(options);
    setProgress(0);
    const collected: Variation[] = [];
    for (const spec of specs) {
      try {
        const opts = { ...options, ...spec.patch };
        const result = await runTrace(source.file, opts);
        const colors = new Set(result.stats.layers.map((l) => l.color));
        collected.push({
          ...spec,
          svg: result.svg,
          colorCount: colors.size,
          pathCount: result.stats.pathCount,
        });
        setVariations([...collected]);
      } catch {
        // A single failed variation shouldn't abort the rest of the grid.
      }
      setProgress((p) => p + 1);
    }
    if (collected.length === 0) {
      setError("Could not generate variations for this image.");
    }
    setRunning(false);
  }

  const total = variationSpecs(options).length;

  return (
    <div className="variations">
      <div className="variations__header">
        <span className="editor__section-title">Variations</span>
        <button type="button" onClick={() => void generate()} disabled={running || !source}>
          {running ? `Generating ${progress}/${total}…` : "Generate"}
        </button>
      </div>

      {error && <p className="variations__error">{error}</p>}

      {variations.length > 0 && (
        <div className="variations__grid">
          {variations.map((variation, i) => (
            <button
              type="button"
              key={i}
              className="variations__cell"
              title="Apply these settings"
              onClick={() => setOptions(variation.patch)}
            >
              <span
                className="variations__thumb"
                dangerouslySetInnerHTML={{ __html: variation.svg }}
              />
              <span className="variations__label">{variation.label}</span>
              <span className="variations__meta">
                {variation.colorCount} colors · {variation.pathCount} paths
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
