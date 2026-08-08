import type { PhysicalTextLine } from "../physical-lines.js";
import {
  findDialogueOccurrences,
  findDualDialogueOccurrences,
  type DialogueOccurrence,
  type DualDialogueOccurrence,
} from "./dialogue-occurrences.js";
import {
  findContextualTransitionCandidates,
  findRepeatedTransitionLines,
  findTransitionLines,
  resolveRepeatedTransitionLines,
} from "./transition-evidence.js";

export interface ScreenplaySequences {
  readonly transitionLines: readonly PhysicalTextLine[];
  readonly dualDialogueOccurrences: readonly DualDialogueOccurrence[];
  readonly dialogueOccurrences: readonly DialogueOccurrence[];
  readonly reservedLines: ReadonlySet<PhysicalTextLine>;
}

export function findScreenplaySequences(
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): ScreenplaySequences {
  const reservedLines = new Set<PhysicalTextLine>();
  const dualDialogueOccurrences = findDualDialogueOccurrences(
    lines,
    coordinateTolerance,
  );

  for (const occurrence of dualDialogueOccurrences) {
    for (const line of occurrence.lines) {
      reservedLines.add(line);
    }
  }

  const dialogueCandidates = findDialogueOccurrences(
    lines,
    reservedLines,
    coordinateTolerance,
  );
  const repeatedTransitionCandidates = findRepeatedTransitionLines(
    lines,
    reservedLines,
    coordinateTolerance,
  );
  const repeatedTransitionLines = resolveRepeatedTransitionLines(
    lines,
    repeatedTransitionCandidates,
    dialogueCandidates,
    coordinateTolerance,
  );
  const contextualTransitionLines = findContextualTransitionCandidates(
    lines,
    reservedLines,
    dialogueCandidates,
    repeatedTransitionLines,
  );
  const arbitratedTransitionLines = [
    ...new Set([...repeatedTransitionLines, ...contextualTransitionLines]),
  ];
  const arbitratedTransitionSet = new Set(arbitratedTransitionLines);
  const dialogueOccurrences = dialogueCandidates.filter(
    (occurrence) => !arbitratedTransitionSet.has(occurrence.characterCue),
  );

  for (const line of arbitratedTransitionLines) {
    reservedLines.add(line);
  }

  for (const occurrence of dialogueOccurrences) {
    reservedLines.add(occurrence.characterCue);

    for (const line of occurrence.dialogueLines) {
      reservedLines.add(line);
    }

    for (const line of occurrence.parentheticals) {
      reservedLines.add(line);
    }
  }

  const transitionLines = findTransitionLines(
    lines,
    reservedLines,
    arbitratedTransitionLines,
    dialogueOccurrences,
    coordinateTolerance,
  );

  for (const line of transitionLines) {
    reservedLines.add(line);
  }

  return {
    transitionLines,
    dualDialogueOccurrences,
    dialogueOccurrences,
    reservedLines,
  };
}
