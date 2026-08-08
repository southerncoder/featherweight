import type { PhysicalTextLine } from "../../core/physical-lines.js";
import {
  isLineInPageMarginBand,
  type PageExtentMeasurements,
} from "../../core/page-extents.js";
import type {
  NormalizationDiagnostic,
  SuppressionReason,
} from "../../core/normalized-text.js";
import {
  addArtifactDiagnostic,
  addSuppressionReason,
} from "./page-artifact-decisions.js";
import {
  createTextFragments,
  isExplicitPaginationText,
  type TextFragment,
} from "./page-artifact-text.js";

export function detectRecurringArtifacts(
  lines: readonly PhysicalTextLine[],
  geometry: PageExtentMeasurements,
  suppressionReasons: Map<PhysicalTextLine, Set<SuppressionReason>>,
  diagnostics: Map<PhysicalTextLine, NormalizationDiagnostic>,
): void {
  const documentPageCount = new Set(lines.map((line) => line.pageIndex)).size;
  const fragmentsByText = Map.groupBy(
    lines.flatMap(createTextFragments).filter(isRecurringArtifactFragment),
    (fragment) => fragment.text,
  );

  for (const fragments of fragmentsByText.values()) {
    const clusters: TextFragment[][] = [];

    for (const fragment of fragments) {
      const cluster = clusters.find((candidate) =>
        haveStablePresentation(candidate[0]!, fragment),
      );

      if (cluster) {
        cluster.push(fragment);
        continue;
      }

      clusters.push([fragment]);
    }

    for (const cluster of clusters) {
      const fragmentsByPage = Map.groupBy(
        cluster,
        (fragment) => fragment.line.pageIndex,
      );
      const unambiguousOccurrences = [...fragmentsByPage.values()]
        .map((pageFragments) => [
          ...new Map(
            pageFragments.map((fragment) => [fragment.line, fragment]),
          ).values(),
        ])
        .filter((pageFragments) => pageFragments.length === 1)
        .map((pageFragments) => pageFragments[0]!);
      const coversDocumentMajority =
        unambiguousOccurrences.length / documentPageCount > 0.5;

      if (unambiguousOccurrences.length < 3 || !coversDocumentMajority) {
        continue;
      }

      for (const fragment of unambiguousOccurrences) {
        if (!isLineInPageMarginBand(fragment.line, geometry)) {
          addArtifactDiagnostic(
            diagnostics,
            fragment.line,
            "recurring-page-artifact",
          );
          continue;
        }

        addSuppressionReason(
          suppressionReasons,
          fragment.line,
          "recurring-page-artifact",
        );
      }
    }
  }
}

function isRecurringArtifactFragment(fragment: TextFragment): boolean {
  if (fragment.text.length < 5) {
    return false;
  }

  return !isExplicitPaginationText(fragment.text);
}

function haveStablePresentation(
  left: TextFragment,
  right: TextFragment,
): boolean {
  const positionTolerance = Math.max(left.fontSize, right.fontSize) / 2;

  return (
    left.fontFamily === right.fontFamily &&
    left.style === right.style &&
    Math.abs(left.fontSize - right.fontSize) <= 0.25 &&
    Math.abs(left.x - right.x) <= positionTolerance &&
    Math.abs(left.y - right.y) <= positionTolerance
  );
}
