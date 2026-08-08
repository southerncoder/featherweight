import type { PhysicalTextLine } from "../../core/physical-lines.js";
import {
  isLineInBottomPageBand,
  isLineInObservedBottomPageBand,
  isLineInTopPageBand,
  pageMarginBandDepth,
  type PageExtentMeasurements,
} from "../../core/page-extents.js";
import type { NormalizationDiagnostic, SuppressionReason } from "../../core/normalized-text.js";
import { addPaginationDiagnostic, addSuppressionReason } from "./page-artifact-decisions.js";
import {
  continuedHeaderPattern,
  continuedMarker,
  createTextFragments,
  moreMarker,
  parseCharacterCue,
  type CharacterCue,
} from "./page-artifact-text.js";

export function detectExplicitPaginationMarkers(
  lines: readonly PhysicalTextLine[],
  geometry: PageExtentMeasurements,
  suppressionReasons: Map<PhysicalTextLine, Set<SuppressionReason>>,
  diagnostics: Map<PhysicalTextLine, NormalizationDiagnostic>,
): void {
  for (const line of lines) {
    const fragments = createTextFragments(line).map((fragment) => fragment.text);
    const lineText = line.text.trim();
    const hasBottomMarker =
      lineText === moreMarker || lineText === continuedMarker;
    const hasTopMarker = fragments.some((text) =>
      continuedHeaderPattern.test(text),
    );

    if (hasBottomMarker) {
      if (isLineInBottomPageBand(line, geometry)) {
        addSuppressionReason(
          suppressionReasons,
          line,
          "screenplay-pagination",
        );
      } else {
        addPaginationDiagnostic(diagnostics, line);
      }
    }

    if (!hasTopMarker) {
      continue;
    }

    if (isLineInTopPageBand(line, geometry)) {
      addSuppressionReason(
        suppressionReasons,
        line,
        "screenplay-pagination",
      );
    } else {
      addPaginationDiagnostic(diagnostics, line);
    }
  }
}

export function detectContinuedCharacterCues(
  lines: readonly PhysicalTextLine[],
  geometry: PageExtentMeasurements,
  suppressionReasons: Map<PhysicalTextLine, Set<SuppressionReason>>,
  diagnostics: Map<PhysicalTextLine, NormalizationDiagnostic>,
): void {
  const linesByPage = Map.groupBy(lines, (line) => line.pageIndex);
  const orderedPageIndexes = [...linesByPage.keys()].sort(
    (left, right) => left - right,
  );

  for (let pageOffset = 1; pageOffset < orderedPageIndexes.length; pageOffset += 1) {
    const pageIndex = orderedPageIndexes[pageOffset]!;
    const previousPageIndex = orderedPageIndexes[pageOffset - 1]!;

    if (pageIndex !== previousPageIndex + 1) {
      continue;
    }

    const pageLines = linesByPage.get(pageIndex)!;
    const previousPageLines = linesByPage.get(previousPageIndex)!;
    const firstContentIndex = pageLines.findIndex(
      (line) => !suppressionReasons.has(line),
    );

    if (firstContentIndex < 0) {
      continue;
    }

    const firstContentLine = pageLines[firstContentIndex]!;
    const currentCue = parseCharacterCue(firstContentLine);

    if (!currentCue || !isLineInTopPageBand(firstContentLine, geometry)) {
      continue;
    }

    const trailingMoreLine = findTrailingMoreLine(
      previousPageLines,
      suppressionReasons,
    );
    const previousContentLine = findPreviousContentLine(
      previousPageLines,
      trailingMoreLine,
      suppressionReasons,
    );
    const followingContentLine = pageLines
      .slice(firstContentIndex + 1)
      .find((line) => !suppressionReasons.has(line));
    const previousCue = findPreviousMatchingColumnCue(
      previousPageLines,
      currentCue,
      suppressionReasons,
    );
    const endsPreviousPage =
      previousContentLine !== undefined &&
      (trailingMoreLine === undefined
        ? isLineInObservedBottomPageBand(previousContentLine, geometry)
        : previousContentLine.bounds.y <=
          trailingMoreLine.bounds.y + pageMarginBandDepth(geometry));
    const hasContinuationContext =
      previousContentLine !== undefined &&
      followingContentLine !== undefined &&
      previousCue?.name === currentCue.name &&
      endsPreviousPage &&
      hasDialogueIndent(firstContentLine, previousContentLine) &&
      hasDialogueIndent(firstContentLine, followingContentLine);

    if (hasContinuationContext) {
      addSuppressionReason(
        suppressionReasons,
        firstContentLine,
        "screenplay-pagination",
      );

      if (trailingMoreLine) {
        addSuppressionReason(
          suppressionReasons,
          trailingMoreLine,
          "screenplay-pagination",
        );
        diagnostics.delete(trailingMoreLine);
      }

      continue;
    }

    if (currentCue.continued) {
      addPaginationDiagnostic(diagnostics, firstContentLine);
    }
  }
}

function findTrailingMoreLine(
  lines: readonly PhysicalTextLine[],
  suppressionReasons: ReadonlyMap<PhysicalTextLine, ReadonlySet<SuppressionReason>>,
): PhysicalTextLine | undefined {
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index]!;

    if (line.text.trim() === moreMarker) {
      return line;
    }

    if (!suppressionReasons.has(line)) {
      return undefined;
    }
  }

  return undefined;
}

function findPreviousContentLine(
  lines: readonly PhysicalTextLine[],
  trailingMoreLine: PhysicalTextLine | undefined,
  suppressionReasons: ReadonlyMap<PhysicalTextLine, ReadonlySet<SuppressionReason>>,
): PhysicalTextLine | undefined {
  const searchEnd = trailingMoreLine
    ? lines.indexOf(trailingMoreLine)
    : lines.length;

  return lines
    .slice(0, searchEnd)
    .findLast((line) => !suppressionReasons.has(line));
}

function findPreviousMatchingColumnCue(
  lines: readonly PhysicalTextLine[],
  currentCue: CharacterCue,
  suppressionReasons: ReadonlyMap<PhysicalTextLine, ReadonlySet<SuppressionReason>>,
): CharacterCue | undefined {
  const currentFontSize = currentCue.line.spans[0]?.font.size ?? 12;

  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index]!;

    if (suppressionReasons.has(line)) {
      continue;
    }

    const cue = parseCharacterCue(line);

    if (!cue) {
      continue;
    }

    if (Math.abs(line.bounds.x - currentCue.line.bounds.x) <= currentFontSize * 2) {
      return cue;
    }
  }

  return undefined;
}

function hasDialogueIndent(
  cueLine: PhysicalTextLine,
  contentLine: PhysicalTextLine,
): boolean {
  const fontSize = cueLine.spans[0]?.font.size ?? 12;
  const indent = cueLine.bounds.x - contentLine.bounds.x;

  return indent >= fontSize * 2 && indent <= fontSize * 8;
}
