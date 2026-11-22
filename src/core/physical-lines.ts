import type {
  PositionedText,
  PositionedTextBounds,
  PositionedTextFont,
  PositionedTextItem,
  PositionedTextStyle,
  SourceMethod,
} from "./positioned-text.js";

export interface PhysicalText {
  readonly lines: readonly PhysicalTextLine[];
}

export interface PhysicalTextLine {
  readonly pageIndex: number;
  readonly text: string;
  readonly bounds: PositionedTextBounds;
  readonly spans: readonly PhysicalTextSpan[];
  /**
   * Leading whitespace characters moved from the text into `bounds.x`, when the
   * PDF indents with spaces instead of positioning text. Absent otherwise.
   */
  readonly indentation?: number;
}

export interface PhysicalTextSpan {
  readonly start: number;
  readonly end: number;
  readonly sourceIndex: number;
  readonly sourceMethod: SourceMethod;
  readonly bounds: PositionedTextBounds;
  readonly font: PositionedTextFont;
  readonly style: PositionedTextStyle;
}

interface BaselineGroup {
  readonly items: PositionedTextItem[];
}

interface ConnectedLineItems {
  readonly items: readonly PositionedTextItem[];
  readonly firstSourceIndex: number;
}

export function groupPositionedTextIntoPhysicalLines(
  positionedText: PositionedText,
): PhysicalText {
  const connectedLines = groupItemsByBaseline(positionedText.items)
    .flatMap((baseline) => splitBaselineByHorizontalConnectivity(baseline.items))
    .sort((left, right) => left.firstSourceIndex - right.firstSourceIndex);
  const spaceWidths = estimateSpaceWidths(positionedText.items);

  return {
    lines: connectedLines.map((line) =>
      createPhysicalTextLine(line.items, spaceWidths),
    ),
  };
}

function groupItemsByBaseline(
  items: readonly PositionedTextItem[],
): readonly BaselineGroup[] {
  const baselines: BaselineGroup[] = [];

  for (const item of items) {
    const compatibleBaseline = baselines.find((baseline) =>
      baseline.items.some((existingItem) =>
        haveCompatibleBaselines(existingItem, item),
      ),
    );

    if (compatibleBaseline) {
      compatibleBaseline.items.push(item);
      continue;
    }

    baselines.push({ items: [item] });
  }

  return baselines;
}

function haveCompatibleBaselines(
  left: PositionedTextItem,
  right: PositionedTextItem,
): boolean {
  if (left.pageIndex !== right.pageIndex) {
    return false;
  }

  const localVerticalScale = Math.min(
    left.bounds.height,
    right.bounds.height,
    left.font.size,
    right.font.size,
  );
  const baselineTolerance = localVerticalScale / 4;

  return Math.abs(left.bounds.y - right.bounds.y) <= baselineTolerance;
}

function splitBaselineByHorizontalConnectivity(
  items: readonly PositionedTextItem[],
): readonly ConnectedLineItems[] {
  const sortedItems = [...items].sort(
    (left, right) =>
      left.bounds.x - right.bounds.x || left.sourceIndex - right.sourceIndex,
  );
  const connectedLines: PositionedTextItem[][] = [];

  for (const item of sortedItems) {
    const currentLine = connectedLines.at(-1);

    if (!currentLine) {
      connectedLines.push([item]);
      continue;
    }

    const previousItem = currentLine.at(-1)!;

    if (areHorizontallyConnected(previousItem, item)) {
      currentLine.push(item);
      continue;
    }

    connectedLines.push([item]);
  }

  return connectedLines.map((lineItems) => ({
    items: lineItems,
    firstSourceIndex: Math.min(
      ...lineItems.map((item) => item.sourceIndex),
    ),
  }));
}

function areHorizontallyConnected(
  left: PositionedTextItem,
  right: PositionedTextItem,
): boolean {
  const horizontalGap = right.bounds.x - (left.bounds.x + left.bounds.width);
  const localRunScale = Math.max(
    left.bounds.height,
    right.bounds.height,
    left.font.size,
    right.font.size,
  );

  return horizontalGap <= localRunScale;
}

function createPhysicalTextLine(
  items: readonly PositionedTextItem[],
  spaceWidths: ReadonlyMap<string, number>,
): PhysicalTextLine {
  let text = "";
  const spans: PhysicalTextSpan[] = [];

  for (const [itemIndex, item] of items.entries()) {
    const previousItem = items[itemIndex - 1];

    if (
      previousItem !== undefined &&
      hasGeometricWordBoundary(previousItem, item)
    ) {
      text += " ";
    }

    // The line's leading indentation is position, not text.
    const itemText =
      itemIndex === 0 ? item.text.slice(leadingIndentation(item)) : item.text;
    const start = text.length;
    text += itemText;
    const end = start + itemText.length;

    spans.push({
      start,
      end,
      sourceIndex: item.sourceIndex,
      sourceMethod: item.sourceMethod,
      bounds: item.bounds,
      font: item.font,
      style: item.style,
    });
  }

  const indentation = leadingIndentation(items[0]!);

  return {
    pageIndex: items[0]!.pageIndex,
    text,
    bounds: createAggregateBounds(items, spaceWidths),
    spans,
    ...(indentation > 0 ? { indentation } : {}),
  };
}

function hasGeometricWordBoundary(
  left: PositionedTextItem,
  right: PositionedTextItem,
): boolean {
  if (/\s$/u.test(left.text) || /^\s/u.test(right.text)) {
    return false;
  }

  if (/^[,.;:!?…)]/u.test(right.text) || left.text.endsWith("-")) {
    return false;
  }

  const horizontalGap = right.bounds.x - (left.bounds.x + left.bounds.width);
  const leftCharacterWidth = left.bounds.width / Math.max(left.text.length, 1);
  const rightCharacterWidth = right.bounds.width / Math.max(right.text.length, 1);
  const localCharacterWidth = Math.min(leftCharacterWidth, rightCharacterWidth);

  return horizontalGap >= localCharacterWidth / 2;
}

function createAggregateBounds(
  items: readonly PositionedTextItem[],
  spaceWidths: ReadonlyMap<string, number>,
): PositionedTextBounds {
  const firstBounds = items[0]!.bounds;
  let minimumX = visibleStartX(items[0]!, spaceWidths);
  let minimumY = firstBounds.y;
  let maximumX = visibleEndX(items[0]!, spaceWidths);
  let maximumY = firstBounds.y + firstBounds.height;

  for (const item of items.slice(1)) {
    minimumX = Math.min(minimumX, item.bounds.x);
    minimumY = Math.min(minimumY, item.bounds.y);
    maximumX = Math.max(maximumX, visibleEndX(item, spaceWidths));
    maximumY = Math.max(maximumY, item.bounds.y + item.bounds.height);
  }

  return {
    x: minimumX,
    y: minimumY,
    width: maximumX - minimumX,
    height: maximumY - minimumY,
  };
}

/**
 * Some PDFs indent with literal spaces instead of positioning text, so every
 * line starts at the same x and only the leading spaces show where it sits.
 * The number of leading whitespace characters that are indentation: all of
 * them, unless the item is whitespace only.
 */
function leadingIndentation(item: PositionedTextItem): number {
  const leading = /^\s+/u.exec(item.text)?.[0].length ?? 0;
  return leading === item.text.length ? 0 : leading;
}

/**
 * Where an item's visible text starts: past its leading indentation, measured
 * in its font's space width.
 */
function visibleStartX(
  item: PositionedTextItem,
  spaceWidths: ReadonlyMap<string, number>,
): number {
  const indentation = leadingIndentation(item);

  if (indentation === 0) {
    return item.bounds.x;
  }

  return item.bounds.x + indentation * spaceWidth(item, spaceWidths);
}

/**
 * Where an item's visible text ends: before its trailing whitespace (line-end
 * padding in text-formatted PDFs), measured in its font's space width.
 */
function visibleEndX(
  item: PositionedTextItem,
  spaceWidths: ReadonlyMap<string, number>,
): number {
  const end = item.bounds.x + item.bounds.width;
  const trailing = /\s+$/u.exec(item.text)?.[0].length ?? 0;

  if (trailing === 0 || trailing === item.text.length) {
    return end;
  }

  return end - trailing * spaceWidth(item, spaceWidths);
}

function spaceWidth(
  item: PositionedTextItem,
  spaceWidths: ReadonlyMap<string, number>,
): number {
  return spaceWidths.get(fontKey(item)) ?? item.bounds.width / item.text.length;
}

function fontKey(item: PositionedTextItem): string {
  return `${item.font.name}\u0000${item.font.size}`;
}

/**
 * Estimates each font's space width from the document, since PDFs report only
 * whole-item widths. Fits `width = space * whitespace + glyph * other` by least
 * squares over every item in the font (exact for monospace fonts, where both
 * widths agree). Fonts without a usable fit are left out, and callers fall back
 * to the item's average character width.
 */
function estimateSpaceWidths(
  items: readonly PositionedTextItem[],
): ReadonlyMap<string, number> {
  const sums = new Map<
    string,
    { aa: number; ab: number; bb: number; aw: number; bw: number }
  >();

  for (const item of items) {
    const whitespace = item.text.match(/\s/gu)?.length ?? 0;
    const other = item.text.length - whitespace;
    const width = item.bounds.width;
    const key = fontKey(item);
    const sum = sums.get(key) ?? { aa: 0, ab: 0, bb: 0, aw: 0, bw: 0 };

    sum.aa += whitespace * whitespace;
    sum.ab += whitespace * other;
    sum.bb += other * other;
    sum.aw += whitespace * width;
    sum.bw += other * width;
    sums.set(key, sum);
  }

  const spaceWidths = new Map<string, number>();

  for (const [key, sum] of sums) {
    const determinant = sum.aa * sum.bb - sum.ab * sum.ab;

    if (determinant <= 0) {
      continue;
    }

    const spaceWidth = (sum.aw * sum.bb - sum.ab * sum.bw) / determinant;

    if (Number.isFinite(spaceWidth) && spaceWidth > 0) {
      spaceWidths.set(key, spaceWidth);
    }
  }

  return spaceWidths;
}
