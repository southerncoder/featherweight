import type { ScreenplayLayout } from "../layout/screenplay-layout.js";
import type { PhysicalTextLine } from "../physical-lines.js";

export function matchesEstablishedRoleGeometry(
  line: PhysicalTextLine,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  const cueTolerance = Math.max(coordinateTolerance, line.bounds.height);

  return (
    matchesEstablishedCueGeometry(line, layout, cueTolerance) ||
    matchesEstablishedContentGeometry(line, layout, coordinateTolerance)
  );
}

export function matchesEstablishedCueGeometry(
  line: PhysicalTextLine,
  layout: ScreenplayLayout,
  tolerance: number,
): boolean {
  const cueAnchors = [
    layout.characterCue,
    layout.dualDialogue?.left.characterCue,
    layout.dualDialogue?.right.characterCue,
  ];

  return cueAnchors.some(
    (anchor) =>
      anchor !== null &&
      anchor !== undefined &&
      Math.abs(line.bounds.x - anchor.x) <= tolerance,
  );
}

export function matchesEstablishedContentGeometry(
  line: PhysicalTextLine,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  const roleTolerance = Math.max(coordinateTolerance, line.bounds.height);
  const contentAnchors = [
    layout.dialogue,
    layout.parenthetical,
    layout.dualDialogue?.left.dialogue,
    layout.dualDialogue?.right.dialogue,
  ];

  if (
    contentAnchors.some(
      (anchor) =>
        anchor !== null &&
        anchor !== undefined &&
        Math.abs(line.bounds.x - anchor.x) <= roleTolerance,
    )
  ) {
    return true;
  }

  return (
    layout.transition !== null &&
    Math.abs(
      line.bounds.x + line.bounds.width - layout.transition.x,
    ) <= roleTolerance
  );
}

export function hasBaselineSibling(
  line: PhysicalTextLine,
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): boolean {
  return lines.some(
    (candidate) =>
      candidate !== line &&
      candidate.pageIndex === line.pageIndex &&
      Math.abs(candidate.bounds.y - line.bounds.y) <= coordinateTolerance,
  );
}

export function isIsolatedFromRoleGeometry(
  line: PhysicalTextLine,
  lines: readonly PhysicalTextLine[],
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  return !lines.some(
    (candidate) =>
      candidate !== line &&
      (areVerticallyAdjacent(candidate, line, 1.5) ||
        areVerticallyAdjacent(line, candidate, 1.5)) &&
      matchesEstablishedRoleGeometry(candidate, layout, coordinateTolerance),
  );
}

export function lineCenter(line: PhysicalTextLine): number {
  return line.bounds.x + line.bounds.width / 2;
}

export function verticalGapInLineHeights(
  upper: PhysicalTextLine,
  lower: PhysicalTextLine,
): number {
  const height = Math.max(upper.bounds.height, lower.bounds.height);

  return (upper.bounds.y - lower.bounds.y) / height;
}

export function areVerticallyAdjacent(
  upper: PhysicalTextLine,
  lower: PhysicalTextLine,
  heightMultiplier: number,
): boolean {
  if (upper.pageIndex !== lower.pageIndex) {
    return false;
  }

  const verticalGap = upper.bounds.y - lower.bounds.y;
  const height = Math.max(upper.bounds.height, lower.bounds.height);

  return verticalGap > 0 && verticalGap <= height * heightMultiplier;
}
