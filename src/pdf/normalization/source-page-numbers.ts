import type { PhysicalTextLine } from "../../core/physical-lines.js";
import {
  getPageHorizontalMidpoint,
  isLineInTopPageBand,
  type PageExtentMeasurements,
} from "../../core/page-extents.js";
import type { SuppressionReason } from "../../core/normalized-text.js";
import { addSuppressionReason } from "./page-artifact-decisions.js";

interface NumberedPageCandidate {
  readonly line: PhysicalTextLine;
  readonly value: number;
}

const pageNumberPattern = /^(\d+)\.$/;

export function detectSourcePageNumbers(
  lines: readonly PhysicalTextLine[],
  geometry: PageExtentMeasurements,
  suppressionReasons: Map<PhysicalTextLine, Set<SuppressionReason>>,
): void {
  const candidates = lines.flatMap((line): readonly NumberedPageCandidate[] => {
    const match = pageNumberPattern.exec(line.text.trim());

    if (!match || !isLineInTopPageBand(line, geometry)) {
      return [];
    }

    const horizontalMidpoint = getPageHorizontalMidpoint(
      geometry,
      line.pageIndex,
    );

    if (horizontalMidpoint === null || line.bounds.x < horizontalMidpoint) {
      return [];
    }

    return [{ line, value: Number(match[1]) }];
  });

  for (let index = 0; index < candidates.length; index += 1) {
    const candidate = candidates[index]!;
    const previous = candidates[index - 1];
    const next = candidates[index + 1];

    if (
      !areSequentialPageNumbers(previous, candidate) &&
      !areSequentialPageNumbers(candidate, next)
    ) {
      continue;
    }

    addSuppressionReason(
      suppressionReasons,
      candidate.line,
      "source-page-number",
    );
  }
}

function areSequentialPageNumbers(
  left: NumberedPageCandidate | undefined,
  right: NumberedPageCandidate | undefined,
): boolean {
  if (!left || !right) {
    return false;
  }

  return (
    right.line.pageIndex === left.line.pageIndex + 1 &&
    right.value === left.value + 1
  );
}
