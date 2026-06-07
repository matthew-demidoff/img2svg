import { useMemo, useState } from "react";
import type { Layer, LayerModel } from "../../lib/layers";
import { LayerRow } from "./LayerRow";

type SortMode = "area" | "original";

const ROW_CAP = 200;

interface LayersPanelProps {
  model: LayerModel;
  disabled: boolean;
  onShowAll: () => void;
  onRecolor: (layerId: string, color: string) => void;
  onToggleVisible: (layer: Layer) => void;
  onSolo: (layerId: string) => void;
  onMoveUp: (layerId: string) => void;
  onMoveDown: (layerId: string) => void;
  onMerge: (layerId: string, intoId: string) => void;
  onDelete: (layerId: string) => void;
  onEyeDropperState: (open: boolean) => void;
}

export function LayersPanel({
  model,
  disabled,
  onShowAll,
  onRecolor,
  onToggleVisible,
  onSolo,
  onMoveUp,
  onMoveDown,
  onMerge,
  onDelete,
  onEyeDropperState,
}: LayersPanelProps) {
  const [sortMode, setSortMode] = useState<SortMode>("area");
  const [query, setQuery] = useState("");
  const [cap, setCap] = useState(ROW_CAP);

  // Reordering only makes sense when the displayed order matches the model's
  // paint order, i.e. unsorted and unfiltered.
  const filter = query.trim().toLowerCase();
  const canReorder = sortMode === "original" && filter === "";

  const filtered = useMemo(() => {
    const indexed = model.layers.map((layer, modelIndex) => ({ layer, modelIndex }));
    const matched = filter
      ? indexed.filter((e) => e.layer.color.toLowerCase().includes(filter))
      : indexed;
    if (sortMode === "area") {
      return [...matched].sort((a, b) => b.layer.count - a.layer.count);
    }
    return matched;
  }, [model.layers, filter, sortMode]);

  // Reset the cap when the matched set changes shape so a fresh search starts
  // from the top.
  const total = model.layers.length;
  const matchCount = filtered.length;
  const visible = filtered.slice(0, cap);
  const hidden = matchCount - visible.length;

  return (
    <div className="editor__layers">
      <div className="editor__layers-header">
        <span className="editor__section-title">Layers</span>
        <button type="button" onClick={onShowAll} disabled={disabled}>
          Show all
        </button>
      </div>

      <div className="editor__layers-controls">
        <input
          type="search"
          className="editor__layers-search"
          placeholder="Filter by hex…"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setCap(ROW_CAP);
          }}
        />
        <select
          className="editor__layers-sort"
          value={sortMode}
          title="Layer order"
          onChange={(e) => {
            setSortMode(e.target.value as SortMode);
            setCap(ROW_CAP);
          }}
        >
          <option value="area">Largest first</option>
          <option value="original">Original order</option>
        </select>
      </div>

      <span className="editor__layers-count">
        {matchCount} of {total} layers
      </span>

      <ul className="editor__layers-list">
        {visible.map(({ layer, modelIndex }) => (
          <LayerRow
            key={layer.id}
            layer={layer}
            index={modelIndex}
            total={total}
            canReorder={canReorder}
            others={model.layers.filter((l) => l.id !== layer.id)}
            onRecolor={(color) => onRecolor(layer.id, color)}
            onToggleVisible={() => onToggleVisible(layer)}
            onSolo={() => onSolo(layer.id)}
            onMoveUp={() => onMoveUp(layer.id)}
            onMoveDown={() => onMoveDown(layer.id)}
            onMerge={(intoId) => onMerge(layer.id, intoId)}
            onDelete={() => onDelete(layer.id)}
            onEyeDropperState={onEyeDropperState}
          />
        ))}
        {hidden > 0 && (
          <li className="editor__layers-more">
            <button type="button" onClick={() => setCap((c) => c + ROW_CAP)}>
              Show {hidden} more
            </button>
          </li>
        )}
      </ul>
    </div>
  );
}
