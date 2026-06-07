import { useStore } from "../store";
import type { ClassOverride, ColorCount, PhotoMode } from "../wasm/coreTypes";

const CLASS_OPTIONS: { value: ClassOverride; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: "logo", label: "Logo" },
  { value: "illustration", label: "Illustration" },
  { value: "photo", label: "Photo" },
];

const PHOTO_OPTIONS: { value: PhotoMode; label: string }[] = [
  { value: "posterize", label: "Posterize" },
  { value: "gradient", label: "Gradient" },
];

const COLOR_OPTIONS: { value: ColorCount; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: 8, label: "8" },
  { value: 16, label: "16" },
  { value: 32, label: "32" },
  { value: 64, label: "64" },
  { value: 128, label: "128" },
  { value: 256, label: "256" },
];

function parseColorCount(raw: string): ColorCount {
  return raw === "auto" ? "auto" : Number(raw);
}

export function Controls() {
  const options = useStore((s) => s.options);
  const setOptions = useStore((s) => s.setOptions);
  const setOptionsDebounced = useStore((s) => s.setOptionsDebounced);
  const disabled = useStore((s) => s.source === null);

  return (
    <fieldset className="controls" disabled={disabled}>
      <label className="control">
        <span>Detail</span>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={options.fidelity}
          onChange={(e) => setOptionsDebounced({ fidelity: Number(e.target.value) })}
        />
        <output>{Math.round(options.fidelity * 100)}%</output>
      </label>

      <label className="control">
        <span>Colors</span>
        <select
          value={String(options.colorCount)}
          onChange={(e) => setOptions({ colorCount: parseColorCount(e.target.value) })}
        >
          {COLOR_OPTIONS.map((o) => (
            <option key={o.label} value={String(o.value)}>
              {o.label}
            </option>
          ))}
        </select>
      </label>

      <label className="control">
        <span>Class</span>
        <select
          value={options.classOverride}
          onChange={(e) => setOptions({ classOverride: e.target.value as ClassOverride })}
        >
          {CLASS_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </label>

      <label className="control">
        <span>Photo mode</span>
        <select
          value={options.photoMode}
          onChange={(e) => setOptions({ photoMode: e.target.value as PhotoMode })}
        >
          {PHOTO_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </label>

      <label className="control control--toggle">
        <input
          type="checkbox"
          checked={options.lockToSourcePalette}
          onChange={(e) => setOptions({ lockToSourcePalette: e.target.checked })}
        />
        <span>Lock to source palette</span>
      </label>

      <label className="control control--toggle">
        <input
          type="checkbox"
          checked={options.blackAndWhite}
          onChange={(e) => setOptions({ blackAndWhite: e.target.checked })}
        />
        <span>Black and white</span>
      </label>
    </fieldset>
  );
}
