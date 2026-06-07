import { useState } from "react";
import { getWorker, useStore } from "../store";

function download(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function baseName(file: File): string {
  return file.name.replace(/\.[^.]+$/, "") || "image";
}

export function ExportButtons() {
  const result = useStore((s) => s.result);
  const source = useStore((s) => s.source);
  const [rendering, setRendering] = useState(false);

  if (!result || !source) {
    return null;
  }

  const { stats } = result;

  function exportSvg() {
    if (!result || !source) {
      return;
    }
    download(new Blob([result.svg], { type: "image/svg+xml" }), `${baseName(source.file)}.svg`);
  }

  async function exportPng() {
    if (!result || !source) {
      return;
    }
    setRendering(true);
    try {
      const blob = await getWorker().renderPng(result.svg, stats.width, stats.height);
      download(blob, `${baseName(source.file)}.png`);
    } finally {
      setRendering(false);
    }
  }

  return (
    <div className="export">
      <button type="button" onClick={exportSvg}>
        Download SVG
      </button>
      <button type="button" onClick={() => void exportPng()} disabled={rendering}>
        {rendering ? "Rendering…" : "Download PNG"}
      </button>
    </div>
  );
}
