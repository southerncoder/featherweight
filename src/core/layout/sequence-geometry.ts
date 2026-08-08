import type { PhysicalTextLine } from "../physical-lines.js";

export function shareBaseline(
  left: PhysicalTextLine,
  right: PhysicalTextLine,
  coordinateTolerance: number,
): boolean {
  return Math.abs(left.bounds.y - right.bounds.y) <= coordinateTolerance;
}

export function isVerticallyAdjacent(
  upper: PhysicalTextLine,
  lower: PhysicalTextLine,
  heightMultiplier: number,
): boolean {
  if (upper.pageIndex !== lower.pageIndex) {
    return false;
  }

  const verticalGap = upper.bounds.y - lower.bounds.y;
  const localHeight = Math.max(upper.bounds.height, lower.bounds.height);

  return verticalGap > 0 && verticalGap <= localHeight * heightMultiplier;
}
