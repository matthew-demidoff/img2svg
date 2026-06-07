import { useEffect, useState } from "react";
import { useStore } from "./store";
import { Dropzone } from "./components/Dropzone";
import { Controls } from "./components/Controls";
import { BeforeAfterSlider } from "./components/BeforeAfterSlider";
import { LayerPreview } from "./components/LayerPreview";
import { StatsBar } from "./components/StatsBar";
import { ExportButtons } from "./components/ExportButtons";
import { Editor } from "./components/Editor";

function useHashRoute(): string {
  const [hash, setHash] = useState(() => window.location.hash);
  useEffect(() => {
    const onChange = () => setHash(window.location.hash);
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return hash;
}

function Home() {
  const status = useStore((s) => s.status);
  const error = useStore((s) => s.error);
  const hasSource = useStore((s) => s.source !== null);
  const hasResult = useStore((s) => s.result !== null);

  return (
    <div className="app">
      <header className="app__header">
        <h1>img2svg</h1>
        <p>Trace a raster image into a layered SVG, entirely in your browser.</p>
      </header>

      <main className="app__main">
        <section className="app__panel">
          <Dropzone />
          <Controls />
          <StatsBar />
          <ExportButtons />
          <button
            type="button"
            className="app__editor-link"
            disabled={!hasResult}
            onClick={() => {
              window.location.hash = "#/editor";
            }}
          >
            Open advanced editor
          </button>
          {status === "processing" && <p className="app__status">Tracing…</p>}
          {status === "error" && error && <p className="app__error">{error}</p>}
        </section>

        {hasSource && (
          <section className="app__stage">
            <BeforeAfterSlider />
            <LayerPreview />
          </section>
        )}
      </main>
    </div>
  );
}

export function App() {
  const hash = useHashRoute();
  if (hash === "#/editor") {
    return <Editor />;
  }
  return <Home />;
}
