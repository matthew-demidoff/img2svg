// An editable model of a traced SVG. VTracer emits a flat list of
// `<path d=".." fill="#RRGGBB" transform="translate(x,y)"/>` in paint order
// (back to front). We fold those into one layer per distinct fill color, keep
// each path's geometry, and can serialize a model back to standalone SVG. The
// model is the single source of truth for both the editor canvas and export.

export interface Shape {
  d: string;
  transform: string | null;
}

export interface Layer {
  id: string;
  /** #rrggbb, lowercased. */
  color: string;
  shapes: Shape[];
  visible: boolean;
}

export interface LayerModel {
  width: number;
  height: number;
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
  // Fall back to the viewBox if width/height are absent.
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
    return { width: 0, height: 0, layers: [] };
  }

  const width = dimension(root, "width");
  const height = dimension(root, "height");

  const byColor = new Map<string, Layer>();
  const order: string[] = [];

  for (const path of root.querySelectorAll("path")) {
    const color = normalizeColor(path.getAttribute("fill"));
    const d = path.getAttribute("d");
    if (!color || !d) {
      continue;
    }
    let layer = byColor.get(color);
    if (!layer) {
      layer = { id: `layer-${order.length}`, color, shapes: [], visible: true };
      byColor.set(color, layer);
      order.push(color);
    }
    layer.shapes.push({ d, transform: path.getAttribute("transform") });
  }

  return { width, height, layers: order.map((c) => byColor.get(c)!) };
}

function escapeAttr(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
}

function shapeMarkup(shape: Shape, color: string): string {
  const transform = shape.transform ? ` transform="${escapeAttr(shape.transform)}"` : "";
  return `<path d="${escapeAttr(shape.d)}" fill="${color}"${transform}/>`;
}

export function serializeLayers(model: LayerModel): string {
  const { width, height } = model;
  const body = model.layers
    .filter((layer) => layer.visible)
    .flatMap((layer) => layer.shapes.map((shape) => shapeMarkup(shape, layer.color)))
    .join("");
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"` +
    ` viewBox="0 0 ${width} ${height}">${body}</svg>`
  );
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
  return { ...model, layers: model.layers.filter((l) => l.id !== layerId) };
}

export function reorder(model: LayerModel, layerId: string, dir: "up" | "down"): LayerModel {
  const index = model.layers.findIndex((l) => l.id === layerId);
  if (index === -1) {
    return model;
  }
  const target = dir === "up" ? index - 1 : index + 1;
  if (target < 0 || target >= model.layers.length) {
    return model;
  }
  const layers = [...model.layers];
  [layers[index], layers[target]] = [layers[target], layers[index]];
  return { ...model, layers };
}

export function merge(model: LayerModel, fromId: string, intoId: string): LayerModel {
  if (fromId === intoId) {
    return model;
  }
  const from = model.layers.find((l) => l.id === fromId);
  const into = model.layers.find((l) => l.id === intoId);
  if (!from || !into) {
    return model;
  }
  const merged = { ...into, shapes: [...into.shapes, ...from.shapes] };
  return {
    ...model,
    layers: model.layers
      .filter((l) => l.id !== fromId)
      .map((l) => (l.id === intoId ? merged : l)),
  };
}
