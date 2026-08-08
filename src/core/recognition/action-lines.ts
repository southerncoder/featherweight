import type { ScreenplayLayout } from "../layout/screenplay-layout.js";
import type { PhysicalTextLine } from "../physical-lines.js";
import type { Action } from "../screenplay-document.js";
import { inferLineMeasure, visibleLineShortfallMultiplier } from "./line-measure.js";
import { areVerticallyAdjacent } from "./recognition-geometry.js";
import { createStyledText, isContinuationDash, type LineJoin } from "./styled-text.js";
import type { SentenceSpacingConvention } from "./sentence-spacing.js";

export interface ActionLineGroup {
  readonly lines: readonly PhysicalTextLine[];
  readonly lineJoins: readonly LineJoin[];
  readonly nextIndex: number;
}

export function collectActionLines(options: {
  readonly lines: readonly PhysicalTextLine[];
  readonly startIndex: number;
  readonly blockedLines: ReadonlySet<PhysicalTextLine>;
  readonly layout: ScreenplayLayout;
  readonly coordinateTolerance: number;
  readonly actionLineMeasure: number | null;
  readonly centeredActionLines: ReadonlySet<PhysicalTextLine>;
}): ActionLineGroup {
  const { lines, startIndex } = options;
  const firstLine = lines[startIndex]!;
  const centered = options.centeredActionLines.has(firstLine);

  if (centered) {
    return collectCenteredActionLines(options);
  }

  const actionLines = [firstLine];
  const lineJoins: LineJoin[] = [];
  let nextIndex = startIndex + 1;

  while (nextIndex < lines.length) {
    const previousLine = actionLines.at(-1)!;
    const nextLine = lines[nextIndex]!;

    if (
      options.blockedLines.has(nextLine) ||
      options.centeredActionLines.has(nextLine) ||
      !haveMatchingActionOrigins(
        previousLine,
        nextLine,
        options.layout,
        options.coordinateTolerance,
      ) ||
      !continuesActionParagraph(
        previousLine,
        nextLine,
        options.actionLineMeasure,
      )
    ) {
      break;
    }

    lineJoins.push(
      shouldPreserveActionLineBreak(
        previousLine,
        nextLine,
        options.actionLineMeasure,
      )
        ? "newline"
        : "wrap",
    );
    actionLines.push(nextLine);
    nextIndex += 1;
  }

  return { lines: actionLines, lineJoins, nextIndex };
}

export function createAction(
  lines: readonly PhysicalTextLine[],
  lineJoins: readonly LineJoin[],
  centeredActionLines: ReadonlySet<PhysicalTextLine>,
  sentenceSpacingConvention: SentenceSpacingConvention,
): Action {
  return {
    type: "action",
    text: createStyledText(lines, lineJoins, sentenceSpacingConvention),
    alignment: centeredActionLines.has(lines[0]!)
      ? "center"
      : "standard",
  };
}

export function inferActionLineMeasure(
  lines: readonly PhysicalTextLine[],
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): number | null {
  if (layout.action === null) {
    return null;
  }

  return inferLineMeasure(lines, layout.action.x, coordinateTolerance);
}

function collectCenteredActionLines(options: {
  readonly lines: readonly PhysicalTextLine[];
  readonly startIndex: number;
  readonly blockedLines: ReadonlySet<PhysicalTextLine>;
  readonly centeredActionLines: ReadonlySet<PhysicalTextLine>;
}): ActionLineGroup {
  const actionLines = [options.lines[options.startIndex]!];
  const lineJoins: LineJoin[] = [];
  let nextIndex = options.startIndex + 1;

  while (nextIndex < options.lines.length) {
    const previousLine = actionLines.at(-1)!;
    const nextLine = options.lines[nextIndex]!;

    if (
      options.blockedLines.has(nextLine) ||
      !options.centeredActionLines.has(nextLine) ||
      !areVerticallyAdjacent(previousLine, nextLine, 1.5)
    ) {
      break;
    }

    lineJoins.push("newline");
    actionLines.push(nextLine);
    nextIndex += 1;
  }

  return { lines: actionLines, lineJoins, nextIndex };
}

function shouldPreserveActionLineBreak(
  previousLine: PhysicalTextLine,
  line: PhysicalTextLine,
  actionLineMeasure: number | null,
): boolean {
  if (actionLineMeasure === null) {
    return false;
  }

  const visibleShortfall =
    Math.min(previousLine.bounds.height, line.bounds.height) *
    visibleLineShortfallMultiplier;

  return (
    previousLine.bounds.width + visibleShortfall < actionLineMeasure &&
    line.bounds.width + visibleShortfall < actionLineMeasure
  );
}

function haveMatchingActionOrigins(
  left: PhysicalTextLine,
  right: PhysicalTextLine,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  if (layout.action === null) {
    return Math.abs(left.bounds.x - right.bounds.x) <= coordinateTolerance;
  }

  const leftMatchesAction =
    Math.abs(left.bounds.x - layout.action.x) <= coordinateTolerance;
  const rightMatchesAction =
    Math.abs(right.bounds.x - layout.action.x) <= coordinateTolerance;

  if (leftMatchesAction || rightMatchesAction) {
    return leftMatchesAction && rightMatchesAction;
  }

  return (
    left.bounds.x > layout.action.x + coordinateTolerance &&
    Math.abs(left.bounds.x - right.bounds.x) <= coordinateTolerance
  );
}

function continuesActionParagraph(
  previousLine: PhysicalTextLine,
  line: PhysicalTextLine,
  actionLineMeasure: number | null,
): boolean {
  if (previousLine.pageIndex === line.pageIndex) {
    return areVerticallyAdjacent(previousLine, line, 1.5);
  }

  if (line.pageIndex !== previousLine.pageIndex + 1) {
    return false;
  }

  if (actionLineMeasure === null) {
    return false;
  }

  const visibleShortfall =
    previousLine.bounds.height * visibleLineShortfallMultiplier;
  const reachesActionMeasure =
    previousLine.bounds.width + visibleShortfall >= actionLineMeasure;
  const previousText = previousLine.text.trimEnd();
  const lineText = line.text.trimStart();
  const hasPairedDashContinuation =
    isContinuationDash(previousText.at(-1) ?? "") &&
    isContinuationDash(lineText[0] ?? "");
  const hasContinuationEnding =
    !/[.!?…]["'’”)\]]?$/u.test(previousText);

  return (
    hasPairedDashContinuation ||
    (reachesActionMeasure && hasContinuationEnding)
  );
}
