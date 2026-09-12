import type { Region } from '../../lib/ipc';
export type Point = { x: number; y: number };
export function boundRegion(region: Region, width: number, height: number): Region {
  const w = Math.min(Math.max(1, Math.round(region.width)), width);
  const h = Math.min(Math.max(1, Math.round(region.height)), height);
  return {
    x: Math.max(0, Math.min(width - w, Math.round(region.x))),
    y: Math.max(0, Math.min(height - h, Math.round(region.y))),
    width: w,
    height: h,
  };
}
export function drawnRegion(a: Point, b: Point, width: number, height: number): Region {
  return boundRegion(
    {
      x: Math.min(a.x, b.x),
      y: Math.min(a.y, b.y),
      width: Math.max(1, Math.abs(a.x - b.x)),
      height: Math.max(1, Math.abs(a.y - b.y)),
    },
    width,
    height,
  );
}
export function editRegion(
  original: Region,
  start: Point,
  end: Point,
  mode: 'move' | 'resize',
  width: number,
  height: number,
): Region {
  if (mode === 'move')
    return boundRegion(
      { ...original, x: original.x + end.x - start.x, y: original.y + end.y - start.y },
      width,
      height,
    );
  return {
    ...original,
    width: Math.max(1, Math.min(width - original.x, Math.round(original.width + end.x - start.x))),
    height: Math.max(
      1,
      Math.min(height - original.y, Math.round(original.height + end.y - start.y)),
    ),
  };
}
