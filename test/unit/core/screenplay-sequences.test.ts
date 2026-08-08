import { describe, expect, it } from "vitest";

import { findScreenplaySequences } from "../../../src/core/layout/screenplay-sequences.js";
import type { PhysicalTextLine } from "../../../src/core/physical-lines.js";

const courierFont = { name: "Courier Prime", size: 12 } as const;
const plainStyle = {
  bold: false,
  italic: false,
  underline: false,
  strikeout: false,
} as const;

function createPhysicalLine(
  sourceIndex: number,
  pageIndex: number,
  text: string,
  x: number,
  y: number,
): PhysicalTextLine {
  const bounds = {
    x,
    y,
    width: text.length * 7.2,
    height: 12,
  };

  return {
    pageIndex,
    text,
    bounds,
    spans: [
      {
        start: 0,
        end: text.length,
        sourceIndex,
        sourceMethod: "embedded-text",
        bounds,
        font: courierFont,
        style: plainStyle,
      },
    ],
  };
}

describe("findScreenplaySequences", () => {
  it("joins dialogue split from the bottom of one page to the top of the next", () => {
    const firstDialogueLine = createPhysicalLine(2, 0, "I heard it", 180, 88);
    const lastPageZeroLine = createPhysicalLine(
      3,
      0,
      "under the tracks.",
      180,
      76,
    );
    const firstPageOneLine = createPhysicalLine(
      4,
      1,
      "It is getting closer.",
      180,
      720,
    );
    const lines = [
      createPhysicalLine(0, 0, "The platform trembles.", 108, 720),
      createPhysicalLine(1, 0, "NOAH", 252, 100),
      firstDialogueLine,
      lastPageZeroLine,
      firstPageOneLine,
      createPhysicalLine(5, 1, "The signal turns red.", 108, 600),
    ];

    const result = findScreenplaySequences(lines, 6);

    expect(result.dialogueOccurrences).toHaveLength(1);
    expect(result.dialogueOccurrences[0]?.dialogueLines).toEqual([
      firstDialogueLine,
      lastPageZeroLine,
      firstPageOneLine,
    ]);
  });

  it("does not continue high dialogue into a same-column mid-page line", () => {
    const dialogueLine = createPhysicalLine(1, 0, "Wait.", 180, 688);
    const midPageLine = createPhysicalLine(
      2,
      1,
      "The station is empty.",
      180,
      360,
    );
    const lines = [
      createPhysicalLine(0, 0, "NOAH", 252, 700),
      dialogueLine,
      midPageLine,
      createPhysicalLine(3, 1, "Rain crosses the tracks.", 108, 300),
      createPhysicalLine(4, 1, "A signal changes.", 108, 72),
    ];

    const result = findScreenplaySequences(lines, 6);

    expect(result.dialogueOccurrences).toHaveLength(1);
    expect(result.dialogueOccurrences[0]?.dialogueLines).toEqual([dialogueLine]);
    expect(result.reservedLines.has(midPageLine)).toBe(false);
  });
});
