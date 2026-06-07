import type { Layer } from "../../lib/layers";
import { getEyeDropper } from "../../lib/eyeDropper";

interface LayerRowProps {
  layer: Layer;
  index: number;
  total: number;
  others: Layer[];
  onRecolor: (color: string) => void;
  onToggleVisible: () => void;
  onSolo: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onMerge: (intoId: string) => void;
  onDelete: () => void;
}

const shapeWord = (n: number) => (n === 1 ? "shape" : "shapes");

export function LayerRow({
  layer,
  index,
  total,
  others,
  onRecolor,
  onToggleVisible,
  onSolo,
  onMoveUp,
  onMoveDown,
  onMerge,
  onDelete,
}: LayerRowProps) {
  const eyeDropper = getEyeDropper();

  async function pickColor() {
    const Ctor = getEyeDropper();
    if (!Ctor) {
      return;
    }
    try {
      const { sRGBHex } = await new Ctor().open();
      onRecolor(sRGBHex.toLowerCase());
    } catch {
      // The user dismissed the picker; leave the color untouched.
    }
  }

  return (
    <li className={layer.visible ? "lrow" : "lrow lrow--off"}>
      <div className="lrow__color">
        <input
          type="color"
          className="lrow__swatch"
          value={layer.color}
          aria-label="Layer color"
          onChange={(e) => onRecolor(e.target.value.toLowerCase())}
        />
        <code className="lrow__hex">{layer.color}</code>
        {eyeDropper && (
          <button
            type="button"
            className="lrow__eyedropper"
            title="Pick color from screen"
            onClick={() => void pickColor()}
          >
            ⊙
          </button>
        )}
      </div>

      <span className="lrow__count">
        {layer.shapes.length} {shapeWord(layer.shapes.length)}
      </span>

      <div className="lrow__actions">
        <button
          type="button"
          aria-pressed={layer.visible}
          title={layer.visible ? "Hide layer" : "Show layer"}
          onClick={onToggleVisible}
        >
          {layer.visible ? "Hide" : "Show"}
        </button>
        <button type="button" title="Show only this layer" onClick={onSolo}>
          Solo
        </button>
        <button type="button" title="Move up" disabled={index === 0} onClick={onMoveUp}>
          ↑
        </button>
        <button
          type="button"
          title="Move down"
          disabled={index === total - 1}
          onClick={onMoveDown}
        >
          ↓
        </button>
        <select
          className="lrow__merge"
          value=""
          title="Merge into another layer"
          disabled={others.length === 0}
          onChange={(e) => {
            if (e.target.value) {
              onMerge(e.target.value);
            }
          }}
        >
          <option value="">Merge into…</option>
          {others.map((other) => (
            <option key={other.id} value={other.id}>
              {other.color}
            </option>
          ))}
        </select>
        <button type="button" className="lrow__delete" title="Delete layer" onClick={onDelete}>
          Delete
        </button>
      </div>
    </li>
  );
}
