import type { PhysicalTextLine } from "../physical-lines.js";
import { clusterCoordinates, median } from "./coordinate-evidence.js";
import type { DialogueOccurrence } from "./dialogue-occurrences.js";
import { isVerticallyAdjacent } from "./sequence-geometry.js";
import { isUppercaseText } from "./screenplay-text.js";

export function findContextualTransitionCandidates(
  lines: readonly PhysicalTextLine[],
  reservedLines: ReadonlySet<PhysicalTextLine>,
  dialogueCandidates: readonly DialogueOccurrence[],
  repeatedTransitionLines: readonly PhysicalTextLine[],
): readonly PhysicalTextLine[] {
  if (dialogueCandidates.length < 2 || repeatedTransitionLines.length > 0) {
    return [];
  }

  const characterCueX = median(
    dialogueCandidates.map((occurrence) => occurrence.characterCue.bounds.x),
  );
  const dialogueX = median(
    dialogueCandidates.flatMap((occurrence) =>
      occurrence.dialogueLines.map((line) => line.bounds.x),
    ),
  );
  const typicalLineHeight = median(lines.map((line) => line.bounds.height));
  const cueToDialogueIndent = characterCueX - dialogueX;
  const rightSideThreshold =
    characterCueX + Math.max(typicalLineHeight * 2, cueToDialogueIndent);
  const lineIndexes = new Map(
    lines.map((line, index) => [line, index] as const),
  );

  return lines.filter((line) => {
    if (
      reservedLines.has(line) ||
      !isUppercaseText(line.text) ||
      line.bounds.x <= rightSideThreshold
    ) {
      return false;
    }

    const nextLine = lines[lineIndexes.get(line)! + 1];

    return (
      nextLine !== undefined &&
      nextLine.pageIndex === line.pageIndex &&
      isVerticallyAdjacent(line, nextLine, 2) &&
      nextLine.bounds.x < line.bounds.x - line.bounds.height
    );
  });
}

export function findTransitionLines(
  lines: readonly PhysicalTextLine[],
  reservedLines: ReadonlySet<PhysicalTextLine>,
  repeatedTransitionLines: readonly PhysicalTextLine[],
  dialogueOccurrences: readonly DialogueOccurrence[],
  coordinateTolerance: number,
): readonly PhysicalTextLine[] {
  if (lines.length === 0) {
    return [];
  }

  const standaloneOpeningTransitions = lines.filter(
    (line, index) =>
      !reservedLines.has(line) &&
      line.text.trim() === "FADE IN" &&
      (index === 0 || lines[index - 1]!.pageIndex !== line.pageIndex),
  );

  const pages = Map.groupBy(lines, (line) => line.pageIndex);
  const pageLeftEdges = [...pages.values()].map((pageLines) =>
    Math.min(...pageLines.map((line) => line.bounds.x)),
  );
  const pageRightEdges = [...pages.values()].map((pageLines) =>
    Math.max(...pageLines.map((line) => line.bounds.x + line.bounds.width)),
  );
  const documentLeftEdge = median(pageLeftEdges);
  const documentRightEdge = median(pageRightEdges);
  const documentMidpoint = (documentLeftEdge + documentRightEdge) / 2;
  const rightSideThreshold = inferRightSideThreshold(
    lines,
    dialogueOccurrences,
    documentMidpoint,
  );
  const geometricCandidates = lines.filter((line) => {
    const rightEdge = line.bounds.x + line.bounds.width;

    return (
      !reservedLines.has(line) &&
      isUppercaseText(line.text) &&
      line.bounds.x > rightSideThreshold &&
      Math.abs(rightEdge - documentRightEdge) <= line.bounds.height
    );
  });
  const rightEdgeClusters = clusterCoordinates(
    geometricCandidates.map((line) => line.bounds.x + line.bounds.width),
    coordinateTolerance,
  );
  const lineIndexes = new Map(
    lines.map((line, index) => [line, index] as const),
  );

  const contextualTransitionLines = geometricCandidates.filter((line) => {
    const lineIndex = lineIndexes.get(line)!;
    const nextLine = lines[lineIndex + 1];
    const hasLeftwardFollowingLine =
      nextLine !== undefined &&
      nextLine.pageIndex === line.pageIndex &&
      isVerticallyAdjacent(line, nextLine, 3) &&
      nextLine.bounds.x < line.bounds.x - line.bounds.height;
    const rightEdge = line.bounds.x + line.bounds.width;
    const hasRepeatedRightEdge = rightEdgeClusters.some(
      (cluster) =>
        cluster.values.length > 1 &&
        cluster.values.some(
          (coordinate) =>
            Math.abs(coordinate - rightEdge) <= coordinateTolerance,
        ),
    );

    return hasLeftwardFollowingLine || hasRepeatedRightEdge;
  });

  return [
    ...new Set([
      ...repeatedTransitionLines,
      ...contextualTransitionLines,
      ...standaloneOpeningTransitions,
    ]),
  ];
}

export function findRepeatedTransitionLines(
  lines: readonly PhysicalTextLine[],
  reservedLines: ReadonlySet<PhysicalTextLine>,
  coordinateTolerance: number,
): readonly PhysicalTextLine[] {
  if (lines.length === 0) {
    return [];
  }

  const typicalLeftEdge = median(lines.map((line) => line.bounds.x));
  const typicalRightEdge = median(
    lines.map((line) => line.bounds.x + line.bounds.width),
  );
  const contentMidpoint = (typicalLeftEdge + typicalRightEdge) / 2;
  const candidates = lines.filter(
    (line) =>
      !reservedLines.has(line) &&
      isUppercaseText(line.text) &&
      line.bounds.x > contentMidpoint,
  );
  return filterRepeatedRightEdgeLines(candidates, coordinateTolerance);
}

function filterRepeatedRightEdgeLines(
  candidates: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): readonly PhysicalTextLine[] {
  const rightEdgeClusters = clusterCoordinates(
    candidates.map((line) => line.bounds.x + line.bounds.width),
    coordinateTolerance,
  );

  return candidates.filter((line) => {
    const rightEdge = line.bounds.x + line.bounds.width;
    const matchingCluster = rightEdgeClusters.find((cluster) =>
      cluster.values.some(
        (coordinate) => Math.abs(coordinate - rightEdge) <= coordinateTolerance,
      ),
    );

    if (matchingCluster === undefined || matchingCluster.values.length < 2) {
      return false;
    }

    const matchingCandidates = candidates.filter((candidate) =>
      matchingCluster.values.some(
        (coordinate) =>
          Math.abs(
            coordinate - (candidate.bounds.x + candidate.bounds.width),
          ) <= coordinateTolerance,
      ),
    );

    return (
      clusterCoordinates(
        matchingCandidates.map((candidate) => candidate.bounds.x),
        coordinateTolerance,
      ).length > 1
    );
  });
}

export function resolveRepeatedTransitionLines(
  lines: readonly PhysicalTextLine[],
  transitionCandidates: readonly PhysicalTextLine[],
  dialogueCandidates: readonly DialogueOccurrence[],
  coordinateTolerance: number,
): readonly PhysicalTextLine[] {
  const transitionCandidateSet = new Set(transitionCandidates);
  const unambiguousDialogueOccurrences = dialogueCandidates.filter(
    (occurrence) => !transitionCandidateSet.has(occurrence.characterCue),
  );

  if (unambiguousDialogueOccurrences.length === 0) {
    return transitionCandidates;
  }

  const characterCueX = median(
    unambiguousDialogueOccurrences.map(
      (occurrence) => occurrence.characterCue.bounds.x,
    ),
  );
  const typicalLineHeight = median(lines.map((line) => line.bounds.height));
  const rightSideThreshold = characterCueX + typicalLineHeight;

  return filterRepeatedRightEdgeLines(
    transitionCandidates.filter((line) => line.bounds.x > rightSideThreshold),
    coordinateTolerance,
  );
}

function inferRightSideThreshold(
  lines: readonly PhysicalTextLine[],
  dialogueOccurrences: readonly DialogueOccurrence[],
  contentMidpoint: number,
): number {
  if (dialogueOccurrences.length === 0) {
    return contentMidpoint;
  }

  const characterCueX = median(
    dialogueOccurrences.map((occurrence) => occurrence.characterCue.bounds.x),
  );
  const typicalLineHeight = median(lines.map((line) => line.bounds.height));

  return Math.max(contentMidpoint, characterCueX + typicalLineHeight);
}
