// An editable model of a traced SVG. VTracer emits a flat list of
// `<path d=".." fill="#RRGGBB" transform="translate(x,y)"/>` in paint order
// (back to front). We keep that exact order in `shapes` so an unedited model
// serializes identically to the source, and expose a per-color `layers` view
// for the UI. The model is the single source of truth for the editor canvas
// and export.

export interface Shape {
  d: string;
  transform: string | null;
  layerId: string;
}

export interface Layer {
  id: string;
  /** #rrggbb, lowercased. */
  color: string;
  visible: boolean;
  count: number;
}

export interface LayerModel {
  width: number;
  height: number;
  /** Paths in original paint order; each tagged with its layer. */
  shapes: Shape[];
  /** One entry per distinct color, in first-seen order. */
  layers: Layer[];
}

const HEX_FILL = /^#[0-9a-f]{6}$/;

function normalizeColor(raw: string | null): string | null {
  if (!raw) {
    return null;
  }
  const color = raw.trim().toLowerCase();
  return HEX_FILL.test(color) ? color : null;
}

function dimension(svg: SVGSVGElement, attr: "width" | "height"): number {
  const value = Number.parseFloat(svg.getAttribute(attr) ?? "");
  if (Number.isFinite(value) && value > 0) {
    return value;
  }
  const box = svg.getAttribute("viewBox")?.split(/[\s,]+/).map(Number);
  if (box && box.length === 4) {
    return attr === "width" ? box[2] : box[3];
  }
  return 0;
}

export function parseLayers(svg: string): LayerModel {
  const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
  const root = doc.querySelector("svg");
  if (!root) {
    return { width: 0, height: 0, shapes: [], layers: [] };
  }

  const width = dimension(root, "width");
  const height = dimension(root, "height");

  const idByColor = new Map<string, string>();
  const layers: Layer[] = [];
  const shapes: Shape[] = [];

  for (const path of root.querySelectorAll("path")) {
    const color = normalizeColor(path.getAttribute("fill"));
    const d = path.getAttribute("d");
    if (!color || !d) {
      continue;
    }
    let layerId = idByColor.get(color);
    if (!layerId) {
      layerId = `layer-${layers.length}`;
      idByColor.set(color, layerId);
      layers.push({ id: layerId, color, visible: true, count: 0 });
    }
    shapes.push({ d, transform: path.getAttribute("transform"), layerId });
  }

  return recount({ width, height, shapes, layers });
}

function escapeAttr(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}

function shapeMarkup(shape: Shape, color: string): string {
  const transform = shape.transform ? ` transform="${escapeAttr(shape.transform)}"` : "";
  return `<path d="${escapeAttr(shape.d)}" fill="${color}"${transform}/>`;
}

export function serializeLayers(model: LayerModel): string {
  const colorOf = new Map<string, string | null>();
  for (const layer of model.layers) {
    colorOf.set(layer.id, layer.visible ? layer.color : null);
  }
  // Emit in original paint order, each shape with its layer's current color,
  // skipping hidden layers. An unedited model therefore round-trips faithfully.
  const body = model.shapes
    .map((shape) => {
      const color = colorOf.get(shape.layerId);
      return color ? shapeMarkup(shape, color) : "";
    })
    .join("");
  const { width, height } = model;
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"` +
    ` viewBox="0 0 ${width} ${height}">${body}</svg>`
  );
}

function recount(model: LayerModel): LayerModel {
  const counts = new Map<string, number>();
  for (const shape of model.shapes) {
    counts.set(shape.layerId, (counts.get(shape.layerId) ?? 0) + 1);
  }
  return {
    ...model,
    layers: model.layers.map((l) => ({ ...l, count: counts.get(l.id) ?? 0 })),
  };
}

function mapLayers(model: LayerModel, fn: (layer: Layer) => Layer): LayerModel {
  return { ...model, layers: model.layers.map(fn) };
}

export function recolor(model: LayerModel, layerId: string, color: string): LayerModel {
  const next = normalizeColor(color);
  if (!next) {
    return model;
  }
  return mapLayers(model, (l) => (l.id === layerId ? { ...l, color: next } : l));
}

export function setVisible(model: LayerModel, layerId: string, visible: boolean): LayerModel {
  return mapLayers(model, (l) => (l.id === layerId ? { ...l, visible } : l));
}

export function solo(model: LayerModel, layerId: string): LayerModel {
  return mapLayers(model, (l) => ({ ...l, visible: l.id === layerId }));
}

export function showAll(model: LayerModel): LayerModel {
  return mapLayers(model, (l) => (l.visible ? l : { ...l, visible: true }));
}

export function remove(model: LayerModel, layerId: string): LayerModel {
  return recount({
    ...model,
    shapes: model.shapes.filter((s) => s.layerId !== layerId),
    layers: model.layers.filter((l) => l.id !== layerId),
  });
}

export function merge(model: LayerModel, fromId: string, intoId: string): LayerModel {
  if (fromId === intoId || !model.layers.some((l) => l.id === intoId)) {
    return model;
  }
  return recount({
    ...model,
    shapes: model.shapes.map((s) => (s.layerId === fromId ? { ...s, layerId: intoId } : s)),
    layers: model.layers.filter((l) => l.id !== fromId),
  });
}

// Move a layer (and all its shapes, as a block) one step in paint order. "up"
// moves it earlier (further back), "down" later (further front), matching the
// top-to-bottom layer list.
export function reorder(model: LayerModel, layerId: string, dir: "up" | "down"): LayerModel {
  const li = model.layers.findIndex((l) => l.id === layerId);
  if (li === -1) {
    return model;
  }
  const ti = dir === "up" ? li - 1 : li + 1;
  if (ti < 0 || ti >= model.layers.length) {
    return model;
  }
  const neighborId = model.layers[ti].id;
  const mine = model.shapes.filter((s) => s.layerId === layerId);
  const rest = model.shapes.filter((s) => s.layerId !== layerId);

  let insertAt: number;
  const firstN = rest.findIndex((s) => s.layerId === neighborId);
  if (firstN === -1) {
    insertAt = dir === "up" ? 0 : rest.length;
  } else if (dir === "up") {
    insertAt = firstN;
  } else {
    let lastN = firstN;
    for (let i = rest.length - 1; i >= 0; i--) {
      if (rest[i].layerId === neighborId) {
        lastN = i;
        break;
      }
    }
    insertAt = lastN + 1;
  }

  const shapes = [...rest.slice(0, insertAt), ...mine, ...rest.slice(insertAt)];
  const layers = [...model.layers];
  [layers[li], layers[ti]] = [layers[ti], layers[li]];
  return { ...model, shapes, layers };
}
