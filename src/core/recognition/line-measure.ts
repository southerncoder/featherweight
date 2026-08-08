import type { PhysicalTextLine } from "../physical-lines.js";

export const visibleLineShortfallMultiplier = 5;

export function inferLineMeasure(
  lines: readonly PhysicalTextLine[],
  anchorX: number,
  coordinateTolerance: number,
): number | null {
  const widths = lines
    .filter(
      (line) =>
        Math.abs(line.bounds.x - anchorX) <= coordinateTolerance,
    )
    .map((line) => line.bounds.width)
    .sort((left, right) => left - right);

  if (widths.length === 0) {
    return null;
  }

  return widths[Math.floor((widths.length - 1) * 0.9)]!;
}
