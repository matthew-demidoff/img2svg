import { useStore } from "../../store";
import type { ClassOverride } from "../../wasm/coreTypes";
import { CLASS_OPTIONS, COLOR_OPTIONS, parseColorCount } from "../../lib/traceOptions";

export function RetracePanel() {
  const options = useStore((s) => s.options);
  const setOptions = useStore((s) => s.setOptions);
  const setOptionsDebounced = useStore((s) => s.setOptionsDebounced);
  const tracing = useStore((s) => s.status === "processing");

  return (
    <div className="retrace">
      <div className="retrace__header">
        <span className="editor__section-title">Re-trace</span>
        {tracing && <span className="retrace__busy">Tracing…</span>}
      </div>

      <label className="retrace__control">
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

      <label className="retrace__control">
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

      <label className="retrace__control">
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
    </div>
  );
}
