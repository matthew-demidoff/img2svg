import { useCallback, useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, ReactNode } from "react";

const MIN_SCALE = 1;
const MAX_SCALE = 16;
const BUTTON_STEP = 1.4;
const DBLCLICK_SCALE = 3;
// exp(-deltaY * sensitivity) per wheel event: proportional, so a mouse notch and
// a fine trackpad swipe both feel smooth instead of jumping a fixed step.
const WHEEL_SENSITIVITY = 0.0018;

interface Transform {
  scale: number;
  x: number;
  y: number;
}

interface Dims {
  vw: number;
  vh: number;
  cw: number;
  ch: number;
}

const clampScale = (s: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s));

// Center the content along an axis when its scaled extent is smaller than the
// viewport; otherwise clamp so the content always covers the viewport. This
// keeps the image from being dragged into empty space and centers letterboxed
// content automatically.
function fitOffset(offset: number, scaled: number, viewport: number): number {
  if (scaled <= viewport + 0.5) {
    return (viewport - scaled) / 2;
  }
  return Math.min(0, Math.max(viewport - scaled, offset));
}

// Anchor the zoom at (fx, fy) in viewport space so the point under the cursor
// stays put, then clamp the result. transform-origin is 0 0 (see styles.css).
function applyZoom(prev: Transform, nextScale: number, fx: number, fy: number, d: Dims): Transform {
  const scale = clampScale(nextScale);
  const ratio = scale / prev.scale;
  return {
    scale,
    x: fitOffset(fx - (fx - prev.x) * ratio, d.cw * scale, d.vw),
    y: fitOffset(fy - (fy - prev.y) * ratio, d.ch * scale, d.vh),
  };
}

export function PanZoom({ children }: { children: ReactNode }) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [t, setT] = useState<Transform>({ scale: 1, x: 0, y: 0 });
  const drag = useRef<{ id: number; startX: number; startY: number; origX: number; origY: number } | null>(null);

  const dims = useCallback((): Dims => {
    const vp = viewportRef.current;
    const ct = contentRef.current;
    const vw = vp?.clientWidth ?? 0;
    const vh = vp?.clientHeight ?? 0;
    return { vw, vh, cw: ct?.offsetWidth || vw, ch: ct?.offsetHeight || vh };
  }, []);

  const focal = useCallback((clientX: number, clientY: number) => {
    const rect = viewportRef.current?.getBoundingClientRect();
    return { fx: clientX - (rect?.left ?? 0), fy: clientY - (rect?.top ?? 0) };
  }, []);

  // Re-center on mount and whenever the viewport or content resizes.
  useEffect(() => {
    const recenter = () =>
      setT((prev) => {
        const d = dims();
        return {
          scale: prev.scale,
          x: fitOffset(prev.x, d.cw * prev.scale, d.vw),
          y: fitOffset(prev.y, d.ch * prev.scale, d.vh),
        };
      });
    recenter();
    const ro = new ResizeObserver(recenter);
    if (viewportRef.current) ro.observe(viewportRef.current);
    if (contentRef.current) ro.observe(contentRef.current);
    return () => ro.disconnect();
  }, [dims]);

  // Wheel must be a non-passive native listener: React's onWheel is passive, so
  // preventDefault there is ignored and the page scrolls while you zoom.
  useEffect(() => {
    const vp = viewportRef.current;
    if (!vp) {
      return;
    }
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const { fx, fy } = focal(e.clientX, e.clientY);
      let delta = e.deltaY;
      if (e.deltaMode === 1) {
        delta *= 16;
      } else if (e.deltaMode === 2) {
        delta *= vp.clientHeight;
      }
      const factor = Math.exp(-delta * WHEEL_SENSITIVITY);
      setT((prev) => applyZoom(prev, prev.scale * factor, fx, fy, dims()));
    };
    vp.addEventListener("wheel", onWheel, { passive: false });
    return () => vp.removeEventListener("wheel", onWheel);
  }, [dims, focal]);

  const isPannable = useCallback(() => {
    const d = dims();
    return d.cw * t.scale > d.vw + 1 || d.ch * t.scale > d.vh + 1;
  }, [dims, t.scale]);

  const onPointerDown = useCallback(
    (e: ReactPointerEvent<HTMLDivElement>) => {
      if (e.button !== 0 || !isPannable()) {
        return;
      }
      e.currentTarget.setPointerCapture(e.pointerId);
      drag.current = { id: e.pointerId, startX: e.clientX, startY: e.clientY, origX: t.x, origY: t.y };
    },
    [isPannable, t.x, t.y],
  );

  const onPointerMove = useCallback(
    (e: ReactPointerEvent<HTMLDivElement>) => {
      const d = drag.current;
      if (!d || d.id !== e.pointerId) {
        return;
      }
      setT((prev) => {
        const dim = dims();
        return {
          ...prev,
          x: fitOffset(d.origX + (e.clientX - d.startX), dim.cw * prev.scale, dim.vw),
          y: fitOffset(d.origY + (e.clientY - d.startY), dim.ch * prev.scale, dim.vh),
        };
      });
    },
    [dims],
  );

  const endDrag = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    if (drag.current?.id === e.pointerId) {
      drag.current = null;
    }
  }, []);

  const onDoubleClick = useCallback(
    (e: ReactPointerEvent<HTMLDivElement>) => {
      const { fx, fy } = focal(e.clientX, e.clientY);
      setT((prev) => {
        const target = prev.scale > MIN_SCALE + 0.01 ? MIN_SCALE : DBLCLICK_SCALE;
        return applyZoom(prev, target, fx, fy, dims());
      });
    },
    [dims, focal],
  );

  const zoomButton = useCallback(
    (factor: number) =>
      setT((prev) => {
        const d = dims();
        return applyZoom(prev, prev.scale * factor, d.vw / 2, d.vh / 2, d);
      }),
    [dims],
  );

  const reset = useCallback(
    () =>
      setT(() => {
        const d = dims();
        return { scale: 1, x: fitOffset(0, d.cw, d.vw), y: fitOffset(0, d.ch, d.vh) };
      }),
    [dims],
  );

  const grab = t.scale > MIN_SCALE + 0.001;

  return (
    <div className="panzoom">
      <div
        ref={viewportRef}
        className={grab ? "panzoom__viewport panzoom__viewport--grab" : "panzoom__viewport"}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onDoubleClick={onDoubleClick}
      >
        <div
          ref={contentRef}
          className="panzoom__content"
          style={{ transform: `translate(${t.x}px, ${t.y}px) scale(${t.scale})` }}
        >
          {children}
        </div>
      </div>
      <div className="panzoom__controls">
        <button type="button" onClick={() => zoomButton(BUTTON_STEP)} aria-label="Zoom in">
          +
        </button>
        <button type="button" onClick={() => zoomButton(1 / BUTTON_STEP)} aria-label="Zoom out">
          −
        </button>
        <button type="button" onClick={reset} aria-label="Reset zoom">
          Reset
        </button>
        <span className="panzoom__level">{Math.round(t.scale * 100)}%</span>
      </div>
    </div>
  );
}
