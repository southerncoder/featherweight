import { describe, expect, it } from "vitest";

import { groupPositionedTextIntoPhysicalLines } from "../../../src/core/physical-lines.js";
import type { PositionedTextItem } from "../../../src/core/positioned-text.js";

// A proportional font whose space is 3 wide and other glyphs 6 wide.
function spaceIndented(sourceIndex: number, text: string, y: number): PositionedTextItem {
  const spaces = text.match(/\s/gu)?.length ?? 0;
  return {
    sourceIndex,
    sourceMethod: "embedded-text",
    pageIndex: 0,
    text,
    bounds: { x: 36, y, width: spaces * 3 + (text.length - spaces) * 6, height: 12 },
    font: { name: "Proportional", size: 12 },
    style: { bold: false, italic: false, underline: false, strikeout: false },
  };
}

describe("space-indented lines", () => {
  it("moves leading indentation into position using the font's space width", () => {
    const { lines } = groupPositionedTextIntoPhysicalLines({
      items: [
        spaceIndented(0, "INT. HOUSE - DAY", 700),
        spaceIndented(1, "                                  MR. BROWN ", 680),
        spaceIndented(2, "                      Like a Virgin.  ", 668),
      ],
    });

    expect(lines.map((line) => [line.text, line.bounds.x])).toEqual([
      ["INT. HOUSE - DAY", 36],
      ["MR. BROWN ", 36 + 34 * 3],
      ["Like a Virgin.  ", 36 + 22 * 3],
    ]);
    expect(lines[1]!.spans[0]).toMatchObject({ start: 0, end: "MR. BROWN ".length });
  });

  it("leaves whitespace-only items unchanged", () => {
    const { lines } = groupPositionedTextIntoPhysicalLines({
      items: [spaceIndented(0, "   ", 700)],
    });

    expect(lines[0]).toMatchObject({ text: "   ", bounds: { x: 36 } });
  });
});
