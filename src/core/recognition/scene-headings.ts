import type { ScreenplayLayout } from "../layout/screenplay-layout.js";
import { isUppercaseText } from "../layout/screenplay-text.js";
import type { PhysicalTextLine } from "../physical-lines.js";
import type { SceneHeading } from "../screenplay-document.js";
import { areVerticallyAdjacent } from "./recognition-geometry.js";
import { visibleLineShortfallMultiplier } from "./line-measure.js";
import { createStyledText, trimStyledTextEnd } from "./styled-text.js";

export interface SceneHeadingEvidence {
  readonly heading: PhysicalTextLine;
  readonly headingLines: readonly PhysicalTextLine[];
  readonly numberFragments: readonly PhysicalTextLine[];
  readonly sceneNumber: string | null;
}

interface SceneNumberEvidence {
  readonly fragments: readonly PhysicalTextLine[];
  readonly number: string;
}

const conventionalSceneHeading = /^(?:INT\.|EXT\.|INT\/EXT\.|I\/E\.|EST\.)/u;
const plausibleSceneNumber = /^(?=.*\d)[A-Z0-9]+(?:[.-][A-Z0-9]+)*$/iu;

export function findSceneHeadings(
  lines: readonly PhysicalTextLine[],
  reservedLines: ReadonlySet<PhysicalTextLine>,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
  actionLineMeasure: number | null,
): readonly SceneHeadingEvidence[] {
  const headings: SceneHeadingEvidence[] = [];
  const consumedContinuationLines = new Set<PhysicalTextLine>();

  for (const [headingIndex, heading] of lines.entries()) {
    if (
      reservedLines.has(heading) ||
      consumedContinuationLines.has(heading)
    ) {
      continue;
    }

    const isEstablishedSceneHeading = isSceneHeadingLine(
      heading,
      layout,
      coordinateTolerance,
    );
    const isPossibleNumberedSceneHeading = isNumberedSceneHeadingLine(
      heading,
      layout,
      coordinateTolerance,
    );

    if (!isEstablishedSceneHeading && !isPossibleNumberedSceneHeading) {
      continue;
    }

    const sceneNumber = findSceneNumberEvidence(
      lines,
      heading,
      coordinateTolerance,
    );

    if (
      !isEstablishedSceneHeading &&
      (sceneNumber === null || !plausibleSceneNumber.test(sceneNumber.number))
    ) {
      continue;
    }

    const continuation = lines[headingIndex + 1];
    const headingLines =
      continuation !== undefined &&
      isWrappedSceneHeadingContinuation(
        heading,
        continuation,
        reservedLines,
        layout,
        coordinateTolerance,
        actionLineMeasure,
      )
        ? [heading, continuation]
        : [heading];

    if (headingLines.length > 1) {
      consumedContinuationLines.add(headingLines[1]!);
    }

    headings.push({
      heading,
      headingLines,
      numberFragments: sceneNumber?.fragments ?? [],
      sceneNumber: sceneNumber?.number ?? null,
    });
  }

  return headings;
}

export function createSceneHeading(
  evidence: SceneHeadingEvidence,
): SceneHeading {
  return {
    type: "scene-heading",
    text: trimStyledTextEnd(createStyledText(evidence.headingLines)),
    sceneNumber: evidence.sceneNumber,
  };
}

function isWrappedSceneHeadingContinuation(
  heading: PhysicalTextLine,
  continuation: PhysicalTextLine,
  reservedLines: ReadonlySet<PhysicalTextLine>,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
  actionLineMeasure: number | null,
): boolean {
  if (
    actionLineMeasure === null ||
    reservedLines.has(continuation) ||
    continuation.pageIndex !== heading.pageIndex ||
    !areVerticallyAdjacent(heading, continuation, 1.5) ||
    layout.action === null ||
    Math.abs(continuation.bounds.x - layout.action.x) > coordinateTolerance ||
    heading.bounds.width +
      heading.bounds.height * visibleLineShortfallMultiplier <
      actionLineMeasure
  ) {
    return false;
  }

  const continuationText = continuation.text.trim();

  return (
    !conventionalSceneHeading.test(continuationText) &&
    isUppercaseText(continuationText) &&
    continuation.spans.length > 0 &&
    continuation.spans.every((span) => span.style.bold)
  );
}

export function isSceneHeadingLine(
  line: PhysicalTextLine,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  if (
    layout.action === null ||
    Math.abs(line.bounds.x - layout.action.x) > coordinateTolerance
  ) {
    return false;
  }

  const trimmedText = line.text.trim();

  return (
    conventionalSceneHeading.test(trimmedText) ||
    (isUppercaseText(trimmedText) &&
      line.spans.length > 0 &&
      line.spans.every((span) => span.style.bold))
  );
}

function isNumberedSceneHeadingLine(
  line: PhysicalTextLine,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  return (
    layout.action !== null &&
    Math.abs(line.bounds.x - layout.action.x) <= coordinateTolerance &&
    isUppercaseText(line.text.trim())
  );
}

function findSceneNumberEvidence(
  lines: readonly PhysicalTextLine[],
  heading: PhysicalTextLine,
  coordinateTolerance: number,
): SceneNumberEvidence | null {
  const baselineFragments = lines.filter(
    (line) =>
      line !== heading &&
      line.pageIndex === heading.pageIndex &&
      Math.abs(line.bounds.y - heading.bounds.y) <= coordinateTolerance,
  );
  const leftFragments = baselineFragments.filter(
    (line) => line.bounds.x < heading.bounds.x,
  );
  const rightFragments = baselineFragments.filter(
    (line) => line.bounds.x > heading.bounds.x + heading.bounds.width,
  );
  const pairedSceneNumber = leftFragments.flatMap((left) =>
    rightFragments
      .filter(
        (right) =>
          right.text.trim() === left.text.trim() &&
          plausibleSceneNumber.test(left.text.trim()),
      )
      .map(
        (right): SceneNumberEvidence => ({
          fragments: [left, right],
          number: left.text.trim(),
        }),
      ),
  )[0];

  if (pairedSceneNumber !== undefined) {
    return pairedSceneNumber;
  }

  const leftSceneNumber = [...leftFragments]
    .filter((line) => plausibleSceneNumber.test(line.text.trim()))
    .sort(
      (left, right) =>
        heading.bounds.x - (left.bounds.x + left.bounds.width) -
        (heading.bounds.x - (right.bounds.x + right.bounds.width)),
    )[0];

  if (leftSceneNumber !== undefined) {
    return {
      fragments: [leftSceneNumber],
      number: leftSceneNumber.text.trim(),
    };
  }

  const rightSceneNumber = [...rightFragments]
    .filter((line) => plausibleSceneNumber.test(line.text.trim()))
    .sort((left, right) => left.bounds.x - right.bounds.x)[0];

  return rightSceneNumber === undefined
    ? null
    : { fragments: [rightSceneNumber], number: rightSceneNumber.text.trim() };
}
