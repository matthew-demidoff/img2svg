import { useEffect, useMemo, useState } from "react";
import { useStore } from "../store";
import { PanZoom } from "./PanZoom";

// Case-insensitive attribute match so we hide paths regardless of whether
// VTracer emitted the fill as #RRGGBB or #rrggbb.
function hideRules(scope: string, colors: Iterable<string>): string {
  const rules: string[] = [];
  for (const color of colors) {
    rules.push(`.${scope} [fill="${color}" i] { display: none; }`);
  }
  return rules.join("\n");
}

export function LayerPreview() {
  const result = useStore((s) => s.result);
  const [hidden, setHidden] = useState<Set<string>>(new Set());

  // A new trace can change the palette; drop selections that no longer apply.
  useEffect(() => {
    setHidden(new Set());
  }, [result?.svg]);

  const scope = "layers-svg";
  const hideCss = useMemo(() => hideRules(scope, hidden), [hidden]);

  if (!result) {
    return null;
  }

  const layers = result.stats.layers;

  function toggle(color: string) {
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(color)) {
        next.delete(color);
      } else {
        next.add(color);
      }
      return next;
    });
  }

  function solo(color: string) {
    setHidden(new Set(layers.map((l) => l.color).filter((c) => c !== color)));
  }

  return (
    <div className="layers">
      <PanZoom>
        <div
          className={scope}
          // The traced SVG is rendered read-only for inspection. It is produced
          // by our own pipeline, not arbitrary user markup.
          dangerouslySetInnerHTML={{ __html: result.svg }}
        />
        {hideCss && <style>{hideCss}</style>}
      </PanZoom>

      <div className="layers__header">
        <span className="layers__title">Layers</span>
        <button
          type="button"
          className="layers__showall"
          onClick={() => setHidden(new Set())}
          disabled={hidden.size === 0}
        >
          Show all
        </button>
      </div>

      <ul className="layers__list">
        {layers.map((layer) => {
          const isHidden = hidden.has(layer.color);
          return (
            <li key={layer.id} className={isHidden ? "layers__item layers__item--off" : "layers__item"}>
              <button
                type="button"
                className="layers__toggle"
                aria-pressed={!isHidden}
                title={isHidden ? "Show layer" : "Hide layer"}
                onClick={() => toggle(layer.color)}
              >
                {isHidden ? "Show" : "Hide"}
              </button>
              <span className="layers__swatch" style={{ background: layer.color }} />
              <code>{layer.color}</code>
              <span className="layers__count">{layer.pathCount} paths</span>
              <button
                type="button"
                className="layers__solo"
                title="Show only this layer"
                onClick={() => solo(layer.color)}
              >
                Solo
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
