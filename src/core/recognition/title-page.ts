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
  /^(?:written|prepared|screenplay|teleplay)\s+(?:by|for)$/iu;
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
  const bodyLines = lines.filter((line) => line.pageIndex !== firstPageIndex);
  const isSeparateTitlePage =
    firstPageLines.length > 0 &&
    bodyLines.length > 0 &&
    bodyLines.some((line) =>
      isSceneHeadingLine(line, layout, coordinateTolerance),
    );

  if (!isSeparateTitlePage) {
    return { fields: [], bodyLines: lines, unattributedLines: [] };
  }

  const fields = recognizeConventionalTitlePage(
    firstPageLines,
    coordinateTolerance,
  );

  if (fields === null) {
    return {
      fields: [],
      bodyLines: lines,
      unattributedLines: firstPageLines,
    };
  }

  return { fields, bodyLines, unattributedLines: [] };
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

  return fields.length > 0 ? fields : null;
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
): readonly TitlePageField[] {
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

  const fields: TitlePageField[] = [];
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
): readonly TitlePageField[] {
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

  const fields: TitlePageField[] = [];
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

function createTitleField(
  key: string,
  lines: readonly PhysicalTextLine[],
): TitlePageField {
  return { key, values: lines.map((line) => createStyledText([line])) };
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

  return Math.abs(lineCenter(left) - lineCenter(right)) <= tolerance;
}
