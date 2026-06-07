import { useEffect, useMemo, useRef, useState } from "react";
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
  type Layer,
  type LayerModel,
} from "../lib/layers";
import { useHistory } from "../lib/useHistory";
import { PanZoom } from "./PanZoom";
import { LayersPanel } from "./editor/LayersPanel";
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

const SHORTCUTS_HELP =
  "Shortcuts: Cmd/Ctrl+Z undo, Cmd/Ctrl+Shift+Z or Cmd/Ctrl+Y redo, Esc to exit.";

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  return tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA" || target.isContentEditable;
}

const EMPTY_MODEL: LayerModel = { width: 0, height: 0, shapes: [], layers: [] };

export function Editor() {
  const result = useStore((s) => s.result);
  const source = useStore((s) => s.source);
  const status = useStore((s) => s.status);

  const history = useHistory<LayerModel>(result ? parseLayers(result.svg) : EMPTY_MODEL);
  const { state: model, canUndo, canRedo, set: setModel, reset, undo, redo } = history;
  const [rendering, setRendering] = useState(false);

  // Latest values the key handler reads without re-binding the listener.
  const eyedropperOpen = useRef(false);
  const actions = useRef({ undo, redo, canUndo, canRedo });
  actions.current = { undo, redo, canUndo, canRedo };

  // Re-seed the editable model whenever the trace changes (re-trace, variation
  // applied, new source). Re-seeding clears undo/redo: it's a new baseline.
  useEffect(() => {
    reset(result ? parseLayers(result.svg) : EMPTY_MODEL);
  }, [result?.svg, reset]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (eyedropperOpen.current || isTypingTarget(e.target)) {
        return;
      }
      if (e.key === "Escape") {
        goHome();
        return;
      }
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) {
        return;
      }
      const key = e.key.toLowerCase();
      const { undo: doUndo, redo: doRedo, canUndo: u, canRedo: r } = actions.current;
      if (key === "z" && !e.shiftKey) {
        e.preventDefault();
        if (u) {
          doUndo();
        }
      } else if ((key === "z" && e.shiftKey) || key === "y") {
        e.preventDefault();
        if (r) {
          doRedo();
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const svg = useMemo(() => serializeLayers(model), [model]);

  if (!result) {
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
    setRendering(true);
    try {
      const blob = await renderPng(svg, model.width, model.height);
      download(blob, `${filenameBase}.png`);
    } finally {
      setRendering(false);
    }
  }

  function resetEdits() {
    reset(result ? parseLayers(result.svg) : EMPTY_MODEL);
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
          <button type="button" onClick={goHome} title="Exit (Esc)">
            Back
          </button>
          <button type="button" disabled={!canUndo} onClick={undo} title="Undo (Cmd/Ctrl+Z)">
            Undo
          </button>
          <button
            type="button"
            disabled={!canRedo}
            onClick={redo}
            title="Redo (Cmd/Ctrl+Shift+Z)"
          >
            Redo
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
          <span className="editor__help" title={SHORTCUTS_HELP} aria-label={SHORTCUTS_HELP}>
            ?
          </span>
        </div>

        <RetracePanel />
        <Variations />

        <LayersPanel
          model={model}
          disabled={status === "processing"}
          onShowAll={() => setModel((m) => showAll(m))}
          onRecolor={(id, color) => setModel((m) => recolor(m, id, color))}
          onToggleVisible={(layer: Layer) =>
            setModel((m) => setVisible(m, layer.id, !layer.visible))
          }
          onSolo={(id) => setModel((m) => solo(m, id))}
          onMoveUp={(id) => setModel((m) => reorder(m, id, "up"))}
          onMoveDown={(id) => setModel((m) => reorder(m, id, "down"))}
          onMerge={(id, intoId) => setModel((m) => merge(m, id, intoId))}
          onDelete={(id) => setModel((m) => remove(m, id))}
          onEyeDropperState={(open) => {
            eyedropperOpen.current = open;
          }}
        />
      </aside>
    </div>
  );
}
