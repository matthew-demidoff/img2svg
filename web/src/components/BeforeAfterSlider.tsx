import { useCallback, useMemo, useRef, useState } from "react";
import { useStore } from "../store";

function svgDataUrl(svg: string): string {
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export function BeforeAfterSlider() {
  const source = useStore((s) => s.source);
  const result = useStore((s) => s.result);
  const [position, setPosition] = useState(50);
  const containerRef = useRef<HTMLDivElement>(null);

  const afterUrl = useMemo(() => (result ? svgDataUrl(result.svg) : null), [result]);

  const updateFromClientX = useCallback((clientX: number) => {
    const el = containerRef.current;
    if (!el) {
      return;
    }
    const rect = el.getBoundingClientRect();
    const ratio = (clientX - rect.left) / rect.width;
    setPosition(Math.min(100, Math.max(0, ratio * 100)));
  }, []);

  if (!source || !afterUrl) {
    return null;
  }

  const aspect = `${source.width} / ${source.height}`;

  return (
    <div
      ref={containerRef}
      className="slider"
      style={{ aspectRatio: aspect }}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        updateFromClientX(e.clientX);
      }}
      onPointerMove={(e) => {
        if (e.buttons === 1) {
          updateFromClientX(e.clientX);
        }
      }}
    >
      <img className="slider__layer" src={source.url} alt="Original raster" draggable={false} />
      <img
        className="slider__layer slider__after"
        style={{ clipPath: `inset(0 ${100 - position}% 0 0)` }}
        src={afterUrl}
        alt="Traced SVG"
        draggable={false}
      />
      <div className="slider__handle" style={{ left: `${position}%` }} aria-hidden="true">
        <span className="slider__grip" />
      </div>
    </div>
  );
}
