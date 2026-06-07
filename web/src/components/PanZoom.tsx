import { useCallback, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, ReactNode, WheelEvent } from "react";

const MIN_SCALE = 1;
const MAX_SCALE = 16;
const ZOOM_STEP = 1.3;

interface Transform {
  scale: number;
  x: number;
  y: number;
}

const IDENTITY: Transform = { scale: 1, x: 0, y: 0 };

function clampScale(scale: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

// Zoom around a focal point in element space so the pixel under the cursor
// stays put: new_offset = focal - (focal - old_offset) * (newScale / oldScale).
function zoomAt(t: Transform, nextScale: number, fx: number, fy: number): Transform {
  const scale = clampScale(nextScale);
  const ratio = scale / t.scale;
  return {
    scale,
    x: fx - (fx - t.x) * ratio,
    y: fy - (fy - t.y) * ratio,
  };
}

export function PanZoom({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  const [t, setT] = useState<Transform>(IDENTITY);
  const drag = useRef<{ pointerId: number; startX: number; startY: number; origX: number; origY: number } | null>(
    null,
  );

  const focalFromEvent = useCallback((clientX: number, clientY: number) => {
    const el = ref.current;
    if (!el) {
      return { fx: 0, fy: 0 };
    }
    const rect = el.getBoundingClientRect();
    return { fx: clientX - rect.left, fy: clientY - rect.top };
  }, []);

  const onWheel = useCallback(
    (e: WheelEvent<HTMLDivElement>) => {
      e.preventDefault();
      const { fx, fy } = focalFromEvent(e.clientX, e.clientY);
      setT((prev) => zoomAt(prev, prev.scale * (e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP), fx, fy));
    },
    [focalFromEvent],
  );

  const onPointerDown = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) {
      return;
    }
    e.currentTarget.setPointerCapture(e.pointerId);
    setT((prev) => {
      drag.current = {
        pointerId: e.pointerId,
        startX: e.clientX,
        startY: e.clientY,
        origX: prev.x,
        origY: prev.y,
      };
      return prev;
    });
  }, []);

  const onPointerMove = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d || d.pointerId !== e.pointerId) {
      return;
    }
    setT((prev) => ({
      ...prev,
      x: d.origX + (e.clientX - d.startX),
      y: d.origY + (e.clientY - d.startY),
    }));
  }, []);

  const endDrag = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    if (drag.current?.pointerId === e.pointerId) {
      drag.current = null;
    }
  }, []);

  const zoomButton = useCallback(
    (factor: number) =>
      setT((prev) => {
        const el = ref.current;
        const fx = el ? el.clientWidth / 2 : 0;
        const fy = el ? el.clientHeight / 2 : 0;
        return zoomAt(prev, prev.scale * factor, fx, fy);
      }),
    [],
  );

  const zoomed = t.scale > 1.001;

  return (
    <div className="panzoom">
      <div
        ref={ref}
        className={zoomed ? "panzoom__viewport panzoom__viewport--grab" : "panzoom__viewport"}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
      >
        <div
          className="panzoom__content"
          style={{ transform: `translate(${t.x}px, ${t.y}px) scale(${t.scale})` }}
        >
          {children}
        </div>
      </div>
      <div className="panzoom__controls">
        <button type="button" onClick={() => zoomButton(ZOOM_STEP)} aria-label="Zoom in">
          +
        </button>
        <button type="button" onClick={() => zoomButton(1 / ZOOM_STEP)} aria-label="Zoom out">
          −
        </button>
        <button type="button" onClick={() => setT(IDENTITY)} aria-label="Reset zoom">
          Reset
        </button>
        <span className="panzoom__level">{Math.round(t.scale * 100)}%</span>
      </div>
    </div>
  );
}
