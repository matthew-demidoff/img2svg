import { useStore } from "../store";

const BYTES_PER_KB = 1024;

function estimateBytes(svg: string): number {
  return new TextEncoder().encode(svg).length;
}

function formatSize(bytes: number): string {
  if (bytes < BYTES_PER_KB) {
    return `${bytes} B`;
  }
  return `${(bytes / BYTES_PER_KB).toFixed(1)} KB`;
}

export function StatsBar() {
  const result = useStore((s) => s.result);
  if (!result) {
    return null;
  }

  const { stats, svg } = result;
  return (
    <div className="stats">
      <span className="stats__item">
        <strong>{stats.pathCount}</strong> paths
      </span>
      <span className="stats__item">
        <strong>{formatSize(estimateBytes(svg))}</strong> estimated
      </span>
      <span className="stats__item">
        class <strong>{stats.detectedClass}</strong>
      </span>
    </div>
  );
}
