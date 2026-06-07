import { useEffect, useMemo, useState } from "react";
import { useStore } from "../store";
import { renderPng } from "../lib/pipeline";
import {
  parseLayers,
  serializeLayers,
  recolor,
  remove,
  reorder,
  merge,
  setVisible,
  solo,
  showAll,
  type LayerModel,
} from "../lib/layers";
import { PanZoom } from "./PanZoom";
import { LayerRow } from "./editor/LayerRow";
import { RetracePanel } from "./editor/RetracePanel";
import { Variations } from "./editor/Variations";

function download(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function baseName(file: File | undefined): string {
  return file?.name.replace(/\.[^.]+$/, "") || "image";
}

function goHome() {
  window.location.hash = "#/";
}

export function Editor() {
  const result = useStore((s) => s.result);
  const source = useStore((s) => s.source);
  const status = useStore((s) => s.status);

  const [model, setModel] = useState<LayerModel | null>(() =>
    result ? parseLayers(result.svg) : null,
  );
  const [rendering, setRendering] = useState(false);

  // Re-seed the editable model whenever the trace changes (re-trace, variation
  // applied, new source). Edits live only in this local model.
  useEffect(() => {
    setModel(result ? parseLayers(result.svg) : null);
  }, [result?.svg]);

  const svg = useMemo(() => (model ? serializeLayers(model) : ""), [model]);

  if (!result || !model) {
    return (
      <div className="editor editor--empty">
        <p>No traced image to edit yet.</p>
        <button type="button" onClick={goHome}>
          Back
        </button>
      </div>
    );
  }

  const filenameBase = baseName(source?.file);

  function downloadSvg() {
    download(new Blob([svg], { type: "image/svg+xml" }), `${filenameBase}.svg`);
  }

  async function downloadPng() {
    if (!model) {
      return;
    }
    setRendering(true);
    try {
      const blob = await renderPng(svg, model.width, model.height);
      download(blob, `${filenameBase}.png`);
    } finally {
      setRendering(false);
    }
  }

  function resetEdits() {
    setModel(result ? parseLayers(result.svg) : null);
  }

  return (
    <div className="editor">
      <div className="editor__canvas">
        <PanZoom>
          <div className="editor__svg" dangerouslySetInnerHTML={{ __html: svg }} />
        </PanZoom>
      </div>

      <aside className="editor__sidebar">
        <div className="editor__toolbar">
          <button type="button" onClick={goHome}>
            Back
          </button>
          <button type="button" onClick={downloadSvg}>
            Download SVG
          </button>
          <button type="button" disabled={rendering} onClick={() => void downloadPng()}>
            {rendering ? "Rendering…" : "Download PNG"}
          </button>
          <button type="button" onClick={resetEdits}>
            Reset edits
          </button>
        </div>

        <RetracePanel />
        <Variations />

        <div className="editor__layers">
          <div className="editor__layers-header">
            <span className="editor__section-title">Layers</span>
            <button
              type="button"
              onClick={() => setModel((m) => (m ? showAll(m) : m))}
              disabled={status === "processing"}
            >
              Show all
            </button>
          </div>
          <ul className="editor__layers-list">
            {model.layers.map((layer, index) => (
              <LayerRow
                key={layer.id}
                layer={layer}
                index={index}
                total={model.layers.length}
                others={model.layers.filter((l) => l.id !== layer.id)}
                onRecolor={(color) => setModel((m) => (m ? recolor(m, layer.id, color) : m))}
                onToggleVisible={() =>
                  setModel((m) => (m ? setVisible(m, layer.id, !layer.visible) : m))
                }
                onSolo={() => setModel((m) => (m ? solo(m, layer.id) : m))}
                onMoveUp={() => setModel((m) => (m ? reorder(m, layer.id, "up") : m))}
                onMoveDown={() => setModel((m) => (m ? reorder(m, layer.id, "down") : m))}
                onMerge={(intoId) => setModel((m) => (m ? merge(m, layer.id, intoId) : m))}
                onDelete={() => setModel((m) => (m ? remove(m, layer.id) : m))}
              />
            ))}
          </ul>
        </div>
      </aside>
    </div>
  );
}
