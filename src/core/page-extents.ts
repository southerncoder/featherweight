import type { PhysicalTextLine } from "./physical-lines.js";

export interface ContentExtents {
  readonly maximumRight: number;
  readonly maximumY: number;
  readonly minimumX: number;
  readonly minimumY: number;
}

export interface PageExtentMeasurements {
  readonly documentExtents: ContentExtents | null;
  readonly pageExtents: ReadonlyMap<number, ContentExtents>;
  readonly pageLineCounts: ReadonlyMap<number, number>;
  readonly typicalFontSize: number;
}

const minimumReliablePageLineCount = 6;
const marginBandFontDepth = 4;

export function measurePageExtents(
  lines: readonly PhysicalTextLine[],
): PageExtentMeasurements {
  const fontSizes = lines
    .flatMap((line) => line.spans.map((span) => span.font.size))
    .sort((left, right) => left - right);
  const typicalFontSize = fontSizes[Math.floor(fontSizes.length / 2)] ?? 12;
  const documentExtents = calculateContentExtents(lines);
  const pageExtents = new Map<number, ContentExtents>();
  const pageLineCounts = new Map<number, number>();
  const linesByPage = Map.groupBy(lines, (line) => line.pageIndex);

  for (const [pageIndex, pageLines] of linesByPage) {
    pageLineCounts.set(pageIndex, pageLines.length);

    const extents = calculateContentExtents(pageLines);

    if (extents) {
      pageExtents.set(pageIndex, extents);
    }
  }

  return {
    documentExtents,
    pageExtents,
    pageLineCounts,
    typicalFontSize,
  };
}

export function getEffectivePageExtents(
  measurements: PageExtentMeasurements,
  pageIndex: number,
): ContentExtents | null {
  const pageLineCount = measurements.pageLineCounts.get(pageIndex) ?? 0;

  if (pageLineCount < minimumReliablePageLineCount) {
    return measurements.documentExtents;
  }

  return measurements.pageExtents.get(pageIndex) ?? measurements.documentExtents;
}

export function getPageHorizontalMidpoint(
  measurements: PageExtentMeasurements,
  pageIndex: number,
): number | null {
  const extents = getEffectivePageExtents(measurements, pageIndex);

  if (!extents) {
    return null;
  }

  return (extents.minimumX + extents.maximumRight) / 2;
}

export function isLineInTopPageBand(
  line: PhysicalTextLine,
  measurements: PageExtentMeasurements,
): boolean {
  const extents = getEffectivePageExtents(measurements, line.pageIndex);

  if (!extents) {
    return false;
  }

  const bandDepth = measurements.typicalFontSize * marginBandFontDepth;
  return line.bounds.y >= extents.maximumY - bandDepth;
}

export function isLineInBottomPageBand(
  line: PhysicalTextLine,
  measurements: PageExtentMeasurements,
): boolean {
  const extents = getEffectivePageExtents(measurements, line.pageIndex);

  if (!extents) {
    return false;
  }

  const bandDepth = measurements.typicalFontSize * marginBandFontDepth;
  return line.bounds.y <= extents.minimumY + bandDepth;
}

export function pageMarginBandDepth(
  measurements: PageExtentMeasurements,
): number {
  return measurements.typicalFontSize * marginBandFontDepth;
}

export function isLineInObservedBottomPageBand(
  line: PhysicalTextLine,
  measurements: PageExtentMeasurements,
): boolean {
  const extents =
    measurements.pageExtents.get(line.pageIndex) ??
    measurements.documentExtents;

  if (!extents) {
    return false;
  }

  return line.bounds.y <= extents.minimumY + pageMarginBandDepth(measurements);
}

export function isLineInPageMarginBand(
  line: PhysicalTextLine,
  measurements: PageExtentMeasurements,
): boolean {
  return (
    isLineInTopPageBand(line, measurements) ||
    isLineInBottomPageBand(line, measurements)
  );
}

function calculateContentExtents(
  lines: readonly PhysicalTextLine[],
): ContentExtents | null {
  const firstLine = lines[0];

  if (!firstLine) {
    return null;
  }

  let maximumRight = firstLine.bounds.x + firstLine.bounds.width;
  let maximumY = firstLine.bounds.y;
  let minimumX = firstLine.bounds.x;
  let minimumY = firstLine.bounds.y;

  for (const line of lines.slice(1)) {
    maximumRight = Math.max(
      maximumRight,
      line.bounds.x + line.bounds.width,
    );
    maximumY = Math.max(maximumY, line.bounds.y);
    minimumX = Math.min(minimumX, line.bounds.x);
    minimumY = Math.min(minimumY, line.bounds.y);
  }

  return { maximumRight, maximumY, minimumX, minimumY };
}
