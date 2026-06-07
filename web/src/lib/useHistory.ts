import { useCallback, useState } from "react";

interface HistoryState<T> {
  past: T[];
  present: T;
  future: T[];
}

const MAX_HISTORY = 100;

export interface History<T> {
  state: T;
  canUndo: boolean;
  canRedo: boolean;
  /** Apply an edit; the previous state becomes undoable and future is dropped. */
  set: (next: T | ((prev: T) => T)) => void;
  /** Re-seed to a new baseline, clearing undo/redo (not an undoable step). */
  reset: (next: T) => void;
  undo: () => void;
  redo: () => void;
}

export function useHistory<T>(initial: T): History<T> {
  const [history, setHistory] = useState<HistoryState<T>>({
    past: [],
    present: initial,
    future: [],
  });

  const set = useCallback((next: T | ((prev: T) => T)) => {
    setHistory((h) => {
      const value = typeof next === "function" ? (next as (p: T) => T)(h.present) : next;
      if (value === h.present) {
        return h;
      }
      const past = [...h.past, h.present];
      if (past.length > MAX_HISTORY) {
        past.shift();
      }
      return { past, present: value, future: [] };
    });
  }, []);

  const reset = useCallback((next: T) => {
    setHistory({ past: [], present: next, future: [] });
  }, []);

  const undo = useCallback(() => {
    setHistory((h) => {
      if (h.past.length === 0) {
        return h;
      }
      const previous = h.past[h.past.length - 1];
      return {
        past: h.past.slice(0, -1),
        present: previous,
        future: [h.present, ...h.future],
      };
    });
  }, []);

  const redo = useCallback(() => {
    setHistory((h) => {
      if (h.future.length === 0) {
        return h;
      }
      const [next, ...rest] = h.future;
      return {
        past: [...h.past, h.present],
        present: next,
        future: rest,
      };
    });
  }, []);

  return {
    state: history.present,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
    set,
    reset,
    undo,
    redo,
  };
}
