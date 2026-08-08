import type { ScreenplayLayout } from "../layout/screenplay-layout.js";
import type { PhysicalTextLine } from "../physical-lines.js";
import {
  areVerticallyAdjacent,
  hasBaselineSibling,
  isIsolatedFromRoleGeometry,
  lineCenter,
  matchesEstablishedContentGeometry,
  matchesEstablishedCueGeometry,
  matchesEstablishedRoleGeometry,
} from "./recognition-geometry.js";

export function findCenteredActionLines(
  lines: readonly PhysicalTextLine[],
  blockedLines: ReadonlySet<PhysicalTextLine>,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): ReadonlySet<PhysicalTextLine> {
  if (layout.action === null) {
    return new Set();
  }

  const candidateLines = lines.filter(
    (line) =>
      !blockedLines.has(line) &&
      line.bounds.x > layout.action!.x + line.bounds.height * 6 &&
      !hasBaselineSibling(line, lines, coordinateTolerance),
  );
  const candidates = candidateLines
    .filter(
      (line) =>
        !matchesEstablishedRoleGeometry(line, layout, coordinateTolerance),
    );
  const candidateSet = new Set(candidateLines);
  const centeredLines = new Set(
    candidates.filter(
      (line) =>
        isAtEstablishedScreenplayCenter(line, layout) &&
        isIsolatedFromRoleGeometry(
          line,
          lines,
          layout,
          coordinateTolerance,
        ),
    ),
  );
  const supportedGroups: {
    readonly center: number;
    readonly upper: PhysicalTextLine;
    readonly lower: PhysicalTextLine;
  }[] = [];

  for (let index = 0; index < lines.length - 1; index += 1) {
    const upper = lines[index]!;
    const lower = lines[index + 1]!;

    if (
      !isCenteredPair(upper, lower, candidateSet, coordinateTolerance) ||
      matchesEstablishedContentGeometry(
        upper,
        layout,
        coordinateTolerance,
      ) ||
      matchesEstablishedContentGeometry(
        lower,
        layout,
        coordinateTolerance,
      ) ||
      (matchesEstablishedCueGeometry(
        upper,
        layout,
        Math.max(coordinateTolerance, upper.bounds.height / 2),
      ) &&
        matchesEstablishedCueGeometry(
          lower,
          layout,
          Math.max(coordinateTolerance, lower.bounds.height / 2),
        ))
    ) {
      continue;
    }

    const groupCenter = (lineCenter(upper) + lineCenter(lower)) / 2;

    if (
      !isCenteredGroupIsolatedFromContentGeometry(
        lines,
        index,
        groupCenter,
        candidateSet,
        layout,
        coordinateTolerance,
      )
    ) {
      continue;
    }

    supportedGroups.push({ center: groupCenter, upper, lower });
    addCenteredGroup(lines, index, groupCenter, candidateSet, centeredLines);
  }

  for (const line of candidates) {
    if (
      isIsolatedFromRoleGeometry(
        line,
        lines,
        layout,
        coordinateTolerance,
      ) &&
      supportedGroups.some(
        (group) =>
          Math.abs(lineCenter(line) - group.center) <= line.bounds.height / 4,
      )
    ) {
      centeredLines.add(line);
    }
  }

  for (const line of candidates) {
    const hasStableCenterPeer = candidates.some(
      (candidate) =>
        candidate !== line &&
        Math.abs(lineCenter(candidate) - lineCenter(line)) <=
          coordinateTolerance,
    );

    if (
      hasStableCenterPeer &&
      isAtEstablishedScreenplayCenter(line, layout) &&
      isIsolatedFromRoleGeometry(
        line,
        lines,
        layout,
        coordinateTolerance,
      )
    ) {
      centeredLines.add(line);
    }
  }

  for (let index = 0; index < lines.length - 1; index += 1) {
    const upper = lines[index]!;
    const lower = lines[index + 1]!;

    if (!isCenteredPair(upper, lower, candidateSet, coordinateTolerance)) {
      continue;
    }

    const groupCenter = (lineCenter(upper) + lineCenter(lower)) / 2;

    if (
      supportedGroups.some(
        (group) =>
          isLocallySupportedCenteredPair(
            upper,
            lower,
            group,
            coordinateTolerance,
          ),
      ) &&
      isCenteredGroupIsolatedFromContentGeometry(
        lines,
        index,
        groupCenter,
        candidateSet,
        layout,
        coordinateTolerance,
      )
    ) {
      addCenteredGroup(lines, index, groupCenter, candidateSet, centeredLines);
    }
  }

  return centeredLines;
}

function isCenteredGroupIsolatedFromContentGeometry(
  lines: readonly PhysicalTextLine[],
  seedIndex: number,
  groupCenter: number,
  candidates: ReadonlySet<PhysicalTextLine>,
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): boolean {
  const range = findCenteredGroupRange(
    lines,
    seedIndex,
    groupCenter,
    candidates,
  );
  const firstLine = lines[range.firstIndex]!;
  const lastLine = lines[range.lastIndex]!;
  const precedingLine = lines[range.firstIndex - 1];
  const followingLine = lines[range.lastIndex + 1];

  return ![precedingLine, followingLine].some(
    (line) =>
      line !== undefined &&
      (areVerticallyAdjacent(line, firstLine, 1.5) ||
        areVerticallyAdjacent(lastLine, line, 1.5)) &&
      matchesEstablishedContentGeometry(line, layout, coordinateTolerance),
  );
}

function isLocallySupportedCenteredPair(
  upper: PhysicalTextLine,
  lower: PhysicalTextLine,
  support: {
    readonly center: number;
    readonly upper: PhysicalTextLine;
    readonly lower: PhysicalTextLine;
  },
  coordinateTolerance: number,
): boolean {
  if (
    upper.pageIndex !== support.upper.pageIndex ||
    Math.abs((lineCenter(upper) + lineCenter(lower)) / 2 - support.center) >
      coordinateTolerance
  ) {
    return false;
  }

  const verticalDistance = Math.min(
    Math.abs(upper.bounds.y - support.upper.bounds.y),
    Math.abs(upper.bounds.y - support.lower.bounds.y),
    Math.abs(lower.bounds.y - support.upper.bounds.y),
    Math.abs(lower.bounds.y - support.lower.bounds.y),
  );
  const lineHeight = Math.max(
    upper.bounds.height,
    lower.bounds.height,
    support.upper.bounds.height,
    support.lower.bounds.height,
  );

  return verticalDistance <= lineHeight * 2.5;
}

function isCenteredPair(
  upper: PhysicalTextLine,
  lower: PhysicalTextLine,
  candidates: ReadonlySet<PhysicalTextLine>,
  coordinateTolerance: number,
): boolean {
  return (
    candidates.has(upper) &&
    candidates.has(lower) &&
    areVerticallyAdjacent(upper, lower, 1.5) &&
    Math.abs(lineCenter(upper) - lineCenter(lower)) <= coordinateTolerance &&
    Math.abs(upper.bounds.x - lower.bounds.x) > coordinateTolerance
  );
}

function addCenteredGroup(
  lines: readonly PhysicalTextLine[],
  seedIndex: number,
  groupCenter: number,
  candidates: ReadonlySet<PhysicalTextLine>,
  centeredLines: Set<PhysicalTextLine>,
): void {
  const range = findCenteredGroupRange(
    lines,
    seedIndex,
    groupCenter,
    candidates,
  );

  for (let index = range.firstIndex; index <= range.lastIndex; index += 1) {
    centeredLines.add(lines[index]!);
  }
}

function findCenteredGroupRange(
  lines: readonly PhysicalTextLine[],
  seedIndex: number,
  groupCenter: number,
  candidates: ReadonlySet<PhysicalTextLine>,
): { readonly firstIndex: number; readonly lastIndex: number } {
  let firstIndex = seedIndex;
  let lastIndex = seedIndex + 1;

  while (
    firstIndex > 0 &&
    belongsToCenteredGroup(
      lines[firstIndex - 1]!,
      lines[firstIndex]!,
      groupCenter,
      candidates,
    )
  ) {
    firstIndex -= 1;
  }

  while (
    lastIndex < lines.length - 1 &&
    belongsToCenteredGroup(
      lines[lastIndex]!,
      lines[lastIndex + 1]!,
      groupCenter,
      candidates,
    )
  ) {
    lastIndex += 1;
  }

  return { firstIndex, lastIndex };
}

function belongsToCenteredGroup(
  upper: PhysicalTextLine,
  lower: PhysicalTextLine,
  groupCenter: number,
  candidates: ReadonlySet<PhysicalTextLine>,
): boolean {
  return (
    candidates.has(upper) &&
    candidates.has(lower) &&
    areVerticallyAdjacent(upper, lower, 1.5) &&
    Math.abs(lineCenter(lower) - groupCenter) <= lower.bounds.height / 4
  );
}

function isAtEstablishedScreenplayCenter(
  line: PhysicalTextLine,
  layout: ScreenplayLayout,
): boolean {
  if (layout.action === null) {
    return false;
  }

  const expectedCenters = [16.5, 18].map(
    (characterMeasure) =>
      layout.action!.x + line.bounds.height * characterMeasure,
  );

  return expectedCenters.some(
    (expectedCenter) =>
      Math.abs(lineCenter(line) - expectedCenter) <= line.bounds.height,
  );
}
