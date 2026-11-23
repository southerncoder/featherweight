import type { ScreenplayLayout } from "../layout/screenplay-layout.js";
import type { PhysicalTextLine } from "../physical-lines.js";
import type { TitlePageField } from "../screenplay-document.js";
import {
  areVerticallyAdjacent,
  lineCenter,
  verticalGapInLineHeights,
} from "./recognition-geometry.js";
import { isSceneHeadingLine } from "./scene-headings.js";
import { createStyledText } from "./styled-text.js";

export interface TitlePageExtraction {
  readonly fields: readonly TitlePageField[];
  readonly bodyLines: readonly PhysicalTextLine[];
  readonly unattributedLines: readonly PhysicalTextLine[];
}

const conventionalCredit =
  /^(?:(?:written|prepared|screenplay|teleplay)\s+(?:by|for)|by)$/iu;
const draftDate = /^\d{1,4}[./-]\d{1,2}[./-]\d{1,4}$/u;
const copyrightNotice = /^(?:copyright\b|©|\(c\))/iu;

export function extractTitlePage(
  lines: readonly PhysicalTextLine[],
  layout: ScreenplayLayout,
  coordinateTolerance: number,
): TitlePageExtraction {
  const actionX = layout.action?.x;

  if (actionX === undefined) {
    return { fields: [], bodyLines: lines, unattributedLines: [] };
  }

  const firstPageIndex = lines[0]!.pageIndex;

  if (firstPageIndex !== 0) {
    return { fields: [], bodyLines: lines, unattributedLines: [] };
  }

  const firstPageLines = lines.filter((line) => line.pageIndex === firstPageIndex);
  const firstPageHeadingIndex = firstPageLines.findIndex((line) =>
    isSceneHeadingLine(line, layout, coordinateTolerance),
  );

  // A title block may share the first page with the script's opening. It is
  // then the lines above the first scene heading, set well apart from it.
  const titleLines =
    firstPageHeadingIndex >= 0
      ? sharedPageTitleLines(firstPageLines, firstPageHeadingIndex)
      : firstPageLines;

  // The script starts at the top of the first page: no title page.
  if (titleLines.length === 0) {
    return { fields: [], bodyLines: lines, unattributedLines: [] };
  }
  const titleLineSet = new Set(titleLines);
  const bodyLines = lines.filter((line) => !titleLineSet.has(line));
  const isSeparateTitlePage =
    titleLines.length > 0 &&
    bodyLines.length > 0 &&
    bodyLines.some((line) =>
      isSceneHeadingLine(line, layout, coordinateTolerance),
    );

  if (!isSeparateTitlePage) {
    return { fields: [], bodyLines: lines, unattributedLines: [] };
  }

  const fields = recognizeConventionalTitlePage(
    titleLines,
    coordinateTolerance,
  );

  if (fields === null) {
    // Lines above the script's first heading that are not a title block are
    // part of the script (cold-open text, scene-number fragments).
    if (firstPageHeadingIndex > 0) {
      return { fields: [], bodyLines: lines, unattributedLines: [] };
    }

    return {
      fields: [],
      bodyLines: lines,
      unattributedLines: titleLines,
    };
  }

  return { fields, bodyLines, unattributedLines: [] };
}

const minimumTitleBlockGap = 3;

/**
 * The lines of a title block that shares the first page with the script: all
 * lines above the first scene heading, when the last of them sits at least
 * {@link minimumTitleBlockGap} line heights above it. Otherwise none, since the
 * lines are part of the script (cold-open text, scene-number fragments).
 */
function sharedPageTitleLines(
  firstPageLines: readonly PhysicalTextLine[],
  headingIndex: number,
): readonly PhysicalTextLine[] {
  const heading = firstPageLines[headingIndex]!;
  const candidates = firstPageLines
    .slice(0, headingIndex)
    .filter((line) => line.bounds.y > heading.bounds.y);
  const lowest = candidates.reduce<PhysicalTextLine | undefined>(
    (current, line) =>
      current === undefined || line.bounds.y < current.bounds.y ? line : current,
    undefined,
  );

  if (
    lowest === undefined ||
    verticalGapInLineHeights(lowest, heading) < minimumTitleBlockGap
  ) {
    return [];
  }

  return candidates;
}

function recognizeConventionalTitlePage(
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): readonly TitlePageField[] | null {
  const footerStartIndex = findTitleFooterStartIndex(lines);
  const upperLines = lines.slice(0, footerStartIndex);
  const footerLines = lines.slice(footerStartIndex);
  const upperFields = recognizeUpperTitleFields(
    upperLines,
    coordinateTolerance,
  );
  let footerFields = recognizeFooterTitleFields(
    footerLines,
    coordinateTolerance,
  );

  if (
    upperFields.length === 0 &&
    footerFields.length === 0 &&
    footerStartIndex === lines.length
  ) {
    footerFields = recognizeFooterTitleFields(lines, coordinateTolerance);
  }

  const fields = [...upperFields, ...footerFields];

  return fields.length > 0 ? toTitlePageFields(lines, fields) : null;
}

function findTitleFooterStartIndex(
  lines: readonly PhysicalTextLine[],
): number {
  let largestGap = 0;
  let footerStartIndex = lines.length;

  for (let index = 1; index < lines.length; index += 1) {
    const gap = verticalGapInLineHeights(lines[index - 1]!, lines[index]!);

    if (gap >= 6 && gap > largestGap) {
      largestGap = gap;
      footerStartIndex = index;
    }
  }

  if (
    footerStartIndex === lines.length &&
    (draftDate.test(lines[0]!.text.trim()) ||
      copyrightNotice.test(lines[0]!.text.trim()))
  ) {
    return 0;
  }

  return footerStartIndex;
}

function recognizeUpperTitleFields(
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): readonly TitleFieldLines[] {
  if (
    lines.length === 0 ||
    !lines.every((line) =>
      haveMatchingCenters(lines[0]!, line, coordinateTolerance),
    )
  ) {
    return [];
  }

  const creditIndex = lines.findIndex((line) =>
    conventionalCredit.test(line.text.trim()),
  );

  if (creditIndex < 0) {
    let titleValueCount = 1;

    while (
      titleValueCount < lines.length &&
      areVerticallyAdjacent(
        lines[titleValueCount - 1]!,
        lines[titleValueCount]!,
        1.5,
      )
    ) {
      titleValueCount += 1;
    }

    return [createTitleField("Title", lines.slice(0, titleValueCount))];
  }

  const fields: TitleFieldLines[] = [];
  const titleValues = lines.slice(0, creditIndex);
  const credit = lines[creditIndex]!;
  const possibleAuthor = lines[creditIndex + 1];
  const hasAuthor =
    possibleAuthor !== undefined &&
    verticalGapInLineHeights(credit, possibleAuthor) <= 2.5;
  const sourceStartIndex = creditIndex + (hasAuthor ? 2 : 1);
  const source = lines.slice(sourceStartIndex);

  if (titleValues.length > 0) {
    fields.push(createTitleField("Title", titleValues));
  }

  fields.push(createTitleField("Credit", [credit]));

  if (hasAuthor) {
    fields.push(createTitleField("Author", [possibleAuthor]));
  }

  if (source.length > 0) {
    fields.push(createTitleField("Source", source));
  }

  return fields;
}

function recognizeFooterTitleFields(
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): readonly TitleFieldLines[] {
  if (lines.length === 0) {
    return [];
  }

  const independentFooter = recognizeIndependentFooter(
    lines,
    coordinateTolerance,
  );

  if (independentFooter !== null) {
    return [
      createTitleField("Notes", independentFooter.notes),
      createTitleField("Copyright", [independentFooter.copyright]),
    ];
  }

  const fields: TitleFieldLines[] = [];
  const copyrightIndex = lines.findIndex((line) =>
    copyrightNotice.test(line.text.trim()),
  );
  const copyright =
    copyrightIndex < 0 ? undefined : lines[copyrightIndex];
  const conventionalLines = lines.filter((_, index) => index !== copyrightIndex);
  const hasDraftDate = draftDate.test(conventionalLines[0]?.text.trim() ?? "");
  const contactCandidate = conventionalLines.slice(hasDraftDate ? 1 : 0);
  const originTolerance = Math.max(
    coordinateTolerance,
    conventionalLines[0]?.bounds.height ?? 0,
  );
  const hasMatchingOrigins = contactCandidate.every(
    (line) =>
      Math.abs(line.bounds.x - contactCandidate[0]!.bounds.x) <=
      originTolerance,
  );
  const hasMatchingRightEdges = contactCandidate.every(
    (line) =>
      Math.abs(
        line.bounds.x +
          line.bounds.width -
          (contactCandidate[0]!.bounds.x + contactCandidate[0]!.bounds.width),
      ) <= originTolerance,
  );
  const hasAdjacentContactLines = contactCandidate
    .slice(1)
    .every((line, index) =>
      areVerticallyAdjacent(contactCandidate[index]!, line, 1.5),
    );
  const hasSupportedContactGeometry = hasDraftDate
    ? contactCandidate.length === 0 ||
      (verticalGapInLineHeights(
        conventionalLines[0]!,
        contactCandidate[0]!,
      ) >= 0.75 &&
        verticalGapInLineHeights(
          conventionalLines[0]!,
          contactCandidate[0]!,
        ) <= 2.5 &&
        hasMatchingOrigins &&
        hasAdjacentContactLines)
    : contactCandidate.length > 1 &&
      hasMatchingOrigins &&
      hasAdjacentContactLines;
  const contact = hasSupportedContactGeometry ? contactCandidate : [];

  if (hasDraftDate) {
    fields.push(createTitleField("Draft date", [conventionalLines[0]!]));
  }

  if (contact.length > 0) {
    fields.push(createTitleField("Contact", contact));
  } else if (
    !hasDraftDate &&
    copyright === undefined &&
    contactCandidate.length > 1 &&
    hasMatchingRightEdges &&
    hasAdjacentContactLines
  ) {
    fields.push(createTitleField("Notes", contactCandidate));
  }

  if (copyright !== undefined) {
    fields.push(createTitleField("Copyright", [copyright]));
  }

  return fields;
}

function recognizeIndependentFooter(
  lines: readonly PhysicalTextLine[],
  coordinateTolerance: number,
): {
  readonly notes: readonly PhysicalTextLine[];
  readonly copyright: PhysicalTextLine;
} | null {
  if (lines.length < 2) {
    return null;
  }

  const copyright = lines.at(-1)!;

  if (!copyrightNotice.test(copyright.text.trim())) {
    return null;
  }

  const notes = lines.slice(0, -1);
  const rightEdgeTolerance = Math.max(
    coordinateTolerance,
    notes[0]!.bounds.height / 2,
  );
  const notesRightEdge = notes[0]!.bounds.x + notes[0]!.bounds.width;
  const hasRightAlignedNotes =
    notes.every(
      (line) =>
        Math.abs(line.bounds.x + line.bounds.width - notesRightEdge) <=
        rightEdgeTolerance,
    ) &&
    notes.slice(1).every((line, index) =>
      areVerticallyAdjacent(notes[index]!, line, 1.5),
    );
  const footerGap = verticalGapInLineHeights(notes.at(-1)!, copyright);
  const isIndependentCopyright =
    footerGap >= 2 &&
    footerGap <= 5 &&
    Math.abs(copyright.bounds.x - notes[0]!.bounds.x) > rightEdgeTolerance;

  return hasRightAlignedNotes && isIndependentCopyright
    ? { notes, copyright }
    : null;
}

/** A title-page field before its lines become styled values. */
interface TitleFieldLines {
  readonly key: string;
  readonly lines: readonly PhysicalTextLine[];
}

function createTitleField(
  key: string,
  lines: readonly PhysicalTextLine[],
): TitleFieldLines {
  return { key, lines };
}

/**
 * Converts recognized fields to title-page fields. Title lines no field claims
 * are kept as Notes, so recognizing a title page never drops text.
 */
function toTitlePageFields(
  lines: readonly PhysicalTextLine[],
  fields: readonly TitleFieldLines[],
): readonly TitlePageField[] {
  const claimed = new Set(fields.flatMap((field) => field.lines));
  const unclaimed = lines.filter((line) => !claimed.has(line));
  const notesIndex = fields.findIndex((field) => field.key === "Notes");
  const completeFields =
    unclaimed.length === 0
      ? fields
      : notesIndex < 0
        ? [...fields, createTitleField("Notes", unclaimed)]
        : fields.map((field, index) =>
            index === notesIndex
              ? createTitleField("Notes", [...field.lines, ...unclaimed])
              : field,
          );

  return completeFields.map((field) => ({
    key: field.key,
    values: field.lines.map((line) => createStyledText([line])),
  }));
}

function haveMatchingCenters(
  left: PhysicalTextLine,
  right: PhysicalTextLine,
  coordinateTolerance: number,
): boolean {
  const tolerance = Math.max(
    coordinateTolerance,
    Math.min(left.bounds.height, right.bounds.height) / 2,
  );

  return (
    Math.abs(lineCenter(left) - lineCenter(right)) <= tolerance ||
    haveMatchingCharacterCenters(left, right)
  );
}

/**
 * Text-formatted PDFs center lines by padding with spaces, which a proportional
 * font no longer renders centered. Compares centers in character columns for
 * two space-indented lines instead.
 */
function haveMatchingCharacterCenters(
  left: PhysicalTextLine,
  right: PhysicalTextLine,
): boolean {
  if (left.indentation === undefined || right.indentation === undefined) {
    return false;
  }

  const characterCenter = (line: PhysicalTextLine): number =>
    line.indentation! + line.text.trimEnd().length / 2;

  return Math.abs(characterCenter(left) - characterCenter(right)) <= 1;
}
