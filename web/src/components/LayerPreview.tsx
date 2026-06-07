import { useStore } from "../store";

export function LayerPreview() {
  const result = useStore((s) => s.result);
  if (!result) {
    return null;
  }

  return (
    <div className="layers">
      <div
        className="layers__svg"
        // The traced SVG is rendered read-only for inspection. It is produced
        // by our own pipeline, not arbitrary user markup.
        dangerouslySetInnerHTML={{ __html: result.svg }}
      />
      <ul className="layers__list">
        {result.stats.layers.map((layer) => (
          <li key={layer.id} className="layers__item">
            <span className="layers__swatch" style={{ background: layer.color }} />
            <code>{layer.color}</code>
            <span className="layers__count">{layer.pathCount} paths</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
