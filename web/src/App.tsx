import { useStore } from "./store";
import { Dropzone } from "./components/Dropzone";
import { Controls } from "./components/Controls";
import { BeforeAfterSlider } from "./components/BeforeAfterSlider";
import { LayerPreview } from "./components/LayerPreview";
import { StatsBar } from "./components/StatsBar";
import { ExportButtons } from "./components/ExportButtons";

export function App() {
  const status = useStore((s) => s.status);
  const error = useStore((s) => s.error);
  const hasSource = useStore((s) => s.source !== null);

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
