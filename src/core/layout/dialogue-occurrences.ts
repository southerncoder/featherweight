import type { PhysicalTextLine } from "../physical-lines.js";
import {
  isLineInBottomPageBand,
  isLineInTopPageBand,
  measurePageExtents,
} from "../page-extents.js";
import { isVerticallyAdjacent, shareBaseline } from "./sequence-geometry.js";
import {
  isCueParentheticalFragment,
  isCueUppercaseText,
  isParenthetical,
  isParentheticalFragment,
} from "./screenplay-text.js";

export interface DualDialogueOccurrence {
  readonly leftCharacterCue: PhysicalTextLine;
  readonly leftDialogue: PhysicalTextLine;
  readonly rightCharacterCue: PhysicalTextLine;
  readonly rightDialogue: PhysicalTextLine;
  readonly leftContent: readonly DialogueContentOccurrence[];
  readonly rightContent: readonly DialogueContentOccurrence[];
  readonly parentheticals: readonly PhysicalTextLine[];
  readonly lines: readonly PhysicalTextLine[];
}

export interface DialogueContentOccurrence {
  readonly type: "parenthetical" | "dialogue";
  readonly lines: readonly PhysicalTextLine[];
}

export interface DialogueOccurrence {
  readonly characterCue: PhysicalTextLine;
  readonly content: readonly DialogueContentOccurrence[];
  readonly dialogueLines: readonly PhysicalTextLine[];
  readonly parentheticals: readonly PhysicalTextLine[];
}


export function findDualDialogueOccurrences(
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): readonly DualDialogueOccurrence[] {
  const occurrences: DualDialogueOccurrence[] = [];

  for (let index = 0; index < lines.length - 1; index += 1) {
    const parsedOccurrence = parseDualDialogueOccurrence(
      lines,
      index,
      coordinateTolerance,
    );

    if (parsedOccurrence === null) {
      continue;
    }

    occurrences.push(parsedOccurrence.occurrence);
    index = parsedOccurrence.nextIndex - 1;
  }

  return occurrences;
}

function parseDualDialogueOccurrence(
  lines: readonly PhysicalTextLine[],
  cueIndex: number,
  coordinateTolerance: number,
): {
  readonly occurrence: DualDialogueOccurrence;
  readonly nextIndex: number;
} | null {
  const leftCharacterCue = lines[cueIndex]!;
  const rightCharacterCue = lines[cueIndex + 1]!;

  if (
    !isCueUppercaseText(leftCharacterCue.text) ||
    !isCueUppercaseText(rightCharacterCue.text) ||
    isCueParentheticalFragment(leftCharacterCue.text) ||
    isCueParentheticalFragment(rightCharacterCue.text) ||
    leftCharacterCue.pageIndex !== rightCharacterCue.pageIndex ||
    !shareBaseline(leftCharacterCue, rightCharacterCue, coordinateTolerance) ||
    leftCharacterCue.bounds.x >= rightCharacterCue.bounds.x
  ) {
    return null;
  }

  const laneBoundary =
    (leftCharacterCue.bounds.x + rightCharacterCue.bounds.x) / 2;
  const occurrenceLines: PhysicalTextLine[] = [
    leftCharacterCue,
    rightCharacterCue,
  ];
  const parentheticals: PhysicalTextLine[] = [];
  let leftDialogue: PhysicalTextLine | undefined;
  let rightDialogue: PhysicalTextLine | undefined;
  let previousBaseline = leftCharacterCue;
  let nextIndex = cueIndex + 2;

  while (nextIndex < lines.length) {
    const row = readBaselineRow(lines, nextIndex, coordinateTolerance);

    if (
      row.lines[0]!.pageIndex !== leftCharacterCue.pageIndex ||
      !isVerticallyAdjacent(previousBaseline, row.lines[0]!, 1.5)
    ) {
      break;
    }

    occurrenceLines.push(...row.lines);

    for (const line of row.lines) {
      if (isParentheticalFragment(line.text)) {
        if (isParenthetical(line.text)) {
          parentheticals.push(line);
        }

        continue;
      }

      if (line.bounds.x < laneBoundary && leftDialogue === undefined) {
        leftDialogue = line;
      }

      if (line.bounds.x > laneBoundary && rightDialogue === undefined) {
        rightDialogue = line;
      }
    }

    previousBaseline = row.lines[0]!;
    nextIndex = row.nextIndex;

    if (leftDialogue !== undefined && rightDialogue !== undefined) {
      break;
    }
  }

  if (
    leftDialogue === undefined ||
    rightDialogue === undefined ||
    leftDialogue.bounds.x >= leftCharacterCue.bounds.x ||
    rightDialogue.bounds.x <= leftCharacterCue.bounds.x ||
    rightDialogue.bounds.x >= rightCharacterCue.bounds.x
  ) {
    return null;
  }

  while (nextIndex < lines.length) {
    const row = readBaselineRow(lines, nextIndex, coordinateTolerance);

    if (
      row.lines[0]!.pageIndex !== leftCharacterCue.pageIndex ||
      !isVerticallyAdjacent(previousBaseline, row.lines[0]!, 1.5) ||
      !row.lines.every((line) =>
        belongsToDualDialogueLane(
          line,
          leftDialogue,
          rightDialogue,
          laneBoundary,
          coordinateTolerance,
        ),
      )
    ) {
      break;
    }

    occurrenceLines.push(...row.lines);
    parentheticals.push(
      ...row.lines.filter((line) => isParenthetical(line.text)),
    );
    previousBaseline = row.lines[0]!;
    nextIndex = row.nextIndex;
  }

  return {
    occurrence: {
      leftCharacterCue,
      leftDialogue,
      rightCharacterCue,
      rightDialogue,
      leftContent: groupDialogueContent(
        occurrenceLines
          .slice(2)
          .filter((line) => line.bounds.x < laneBoundary),
      ),
      rightContent: groupDialogueContent(
        occurrenceLines
          .slice(2)
          .filter((line) => line.bounds.x > laneBoundary),
      ),
      parentheticals,
      lines: occurrenceLines,
    },
    nextIndex,
  };
}

function readBaselineRow(
  lines: readonly PhysicalTextLine[],
  startIndex: number,
  coordinateTolerance: number,
): { readonly lines: readonly PhysicalTextLine[]; readonly nextIndex: number } {
  const firstLine = lines[startIndex]!;
  const rowLines: PhysicalTextLine[] = [firstLine];
  let nextIndex = startIndex + 1;

  while (
    nextIndex < lines.length &&
    lines[nextIndex]!.pageIndex === firstLine.pageIndex &&
    shareBaseline(firstLine, lines[nextIndex]!, coordinateTolerance)
  ) {
    rowLines.push(lines[nextIndex]!);
    nextIndex += 1;
  }

  return { lines: rowLines, nextIndex };
}

function belongsToDualDialogueLane(
  line: PhysicalTextLine,
  leftDialogue: PhysicalTextLine,
  rightDialogue: PhysicalTextLine,
  laneBoundary: number,
  coordinateTolerance: number,
): boolean {
  if (isParentheticalFragment(line.text)) {
    return true;
  }

  const expectedX =
    line.bounds.x < laneBoundary
      ? leftDialogue.bounds.x
      : rightDialogue.bounds.x;

  return Math.abs(line.bounds.x - expectedX) <= coordinateTolerance;
}

export function findDialogueOccurrences(
  lines: readonly PhysicalTextLine[],
  reservedLines: ReadonlySet<PhysicalTextLine>,
  coordinateTolerance: number,
): readonly DialogueOccurrence[] {
  const occurrences: DialogueOccurrence[] = [];
  const pageExtents = measurePageExtents(lines);

  for (let index = 0; index < lines.length - 1; index += 1) {
    const characterCue = lines[index]!;

    if (
      reservedLines.has(characterCue) ||
      isCueParentheticalFragment(characterCue.text)
    ) {
      continue;
    }

    let dialogueIndex = index + 1;
    const contentLines: PhysicalTextLine[] = [];
    let previousContentLine = characterCue;
    let parentheticalOpen = false;

    while (dialogueIndex < lines.length) {
      const possibleParenthetical = lines[dialogueIndex]!;
      const continuesParenthetical =
        parentheticalOpen ||
        possibleParenthetical.text.trimStart().startsWith("(");

      if (
        !continuesParenthetical ||
        reservedLines.has(possibleParenthetical) ||
        possibleParenthetical.pageIndex !== characterCue.pageIndex ||
        !isVerticallyAdjacent(previousContentLine, possibleParenthetical, 1.5) ||
        possibleParenthetical.bounds.x >= characterCue.bounds.x
      ) {
        break;
      }

      contentLines.push(possibleParenthetical);
      parentheticalOpen = !possibleParenthetical.text.trimEnd().endsWith(")");
      previousContentLine = possibleParenthetical;
      dialogueIndex += 1;
    }

    if (parentheticalOpen) {
      continue;
    }

    const firstDialogueLine = lines[dialogueIndex];
    const precedingLine = lines[dialogueIndex - 1]!;

    if (
      firstDialogueLine === undefined ||
      reservedLines.has(firstDialogueLine) ||
      firstDialogueLine.pageIndex !== characterCue.pageIndex ||
      !isVerticallyAdjacent(precedingLine, firstDialogueLine, 1.5) ||
      characterCue.bounds.x - firstDialogueLine.bounds.x <=
        characterCue.bounds.height ||
      (!isCueUppercaseText(characterCue.text) &&
        characterCue.bounds.width >= firstDialogueLine.bounds.width)
    ) {
      continue;
    }

    const dialogueLines: PhysicalTextLine[] = [firstDialogueLine];
    contentLines.push(firstDialogueLine);
    let previousLine = firstDialogueLine;
    let continuationIndex = dialogueIndex + 1;
    parentheticalOpen = false;

    while (continuationIndex < lines.length) {
      const continuation = lines[continuationIndex]!;

      if (reservedLines.has(continuation)) {
        break;
      }

      const continuesParenthetical =
        parentheticalOpen || continuation.text.trimStart().startsWith("(");

      if (
        continuesParenthetical &&
        continuation.pageIndex === previousLine.pageIndex &&
        isVerticallyAdjacent(previousLine, continuation, 1.5) &&
        continuation.bounds.x > firstDialogueLine.bounds.x &&
        continuation.bounds.x < characterCue.bounds.x
      ) {
        contentLines.push(continuation);
        parentheticalOpen = !continuation.text.trimEnd().endsWith(")");
        previousLine = continuation;
        continuationIndex += 1;
        continue;
      }

      const continuesAcrossPage =
        continuation.pageIndex === previousLine.pageIndex + 1 &&
        isLineInBottomPageBand(previousLine, pageExtents) &&
        isLineInTopPageBand(continuation, pageExtents);
      const continuesOnPage =
        continuation.pageIndex === previousLine.pageIndex &&
        isVerticallyAdjacent(previousLine, continuation, 1.5);

      if (
        Math.abs(continuation.bounds.x - firstDialogueLine.bounds.x) >
          coordinateTolerance ||
        (!continuesAcrossPage && !continuesOnPage)
      ) {
        break;
      }

      dialogueLines.push(continuation);
      contentLines.push(continuation);
      previousLine = continuation;
      continuationIndex += 1;
    }

    const content = groupDialogueContent(contentLines);
    occurrences.push({
      characterCue,
      content,
      dialogueLines,
      parentheticals: content
        .filter((item) => item.type === "parenthetical")
        .flatMap((item) => item.lines),
    });
    index = continuationIndex - 1;
  }

  return occurrences;
}

function groupDialogueContent(
  lines: readonly PhysicalTextLine[],
): readonly DialogueContentOccurrence[] {
  const groups: Array<{
    readonly type: DialogueContentOccurrence["type"];
    readonly lines: PhysicalTextLine[];
  }> = [];
  let parentheticalOpen = false;

  for (const line of lines) {
    const type: DialogueContentOccurrence["type"] =
      parentheticalOpen || line.text.trimStart().startsWith("(")
        ? "parenthetical"
        : "dialogue";
    const currentGroup = groups.at(-1);

    if (currentGroup !== undefined && currentGroup.type === type) {
      currentGroup.lines.push(line);
    } else {
      groups.push({ type, lines: [line] });
    }

    parentheticalOpen =
      type === "parenthetical" && !line.text.trimEnd().endsWith(")");
  }

  return groups;
}
