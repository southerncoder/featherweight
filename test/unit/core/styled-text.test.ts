import { describe, expect, it } from "vitest";

import type { PhysicalTextLine } from "../../../src/core/physical-lines.js";
import {
  createStyledText,
  sliceStyledText,
  trimStyledTextEnd,
} from "../../../src/core/recognition/styled-text.js";
import { run, styled } from "../../fixtures/semantic/document-builders.js";

describe("styled text", () => {
  it("slices within a run while preserving its styles", () => {
    const text = styled(run("Bold", ["bold"]), run(" italic", ["italic"]));

    expect(sliceStyledText(text, 7)).toEqual(
      styled(run("Bold", ["bold"]), run(" it", ["italic"])),
    );
  });

  it("returns all runs when the slice end exceeds the text length", () => {
    const text = styled(run("Bold", ["bold"]), run(" italic", ["italic"]));

    expect(sliceStyledText(text, 100)).toEqual(text);
  });

  it("trims trailing whitespace across multiple runs", () => {
    const text = styled(
      run("Text", ["bold"]),
      run("  ", ["italic"]),
      run("\t", ["underline"]),
    );

    expect(trimStyledTextEnd(text)).toEqual(styled(run("Text", ["bold"])));
  });

  it("trims all-whitespace text to no runs", () => {
    const text = styled(run(" ", ["bold"]), run("\t", ["italic"]));

    expect(trimStyledTextEnd(text)).toEqual(styled());
  });

  it("does not create empty text runs", () => {
    const bounds = { x: 108, y: 700, width: 0, height: 12 };
    const line: PhysicalTextLine = {
      pageIndex: 0,
      text: "",
      bounds,
      spans: [
        {
          start: 0,
          end: 0,
          sourceIndex: 0,
          sourceMethod: "embedded-text",
          bounds,
          font: { name: "Courier Prime", size: 12 },
          style: {
            bold: false,
            italic: false,
            underline: false,
            strikeout: false,
          },
        },
      ],
    };

    expect(createStyledText([line])).toEqual(styled());
  });
});
