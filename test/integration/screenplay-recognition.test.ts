import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { inferScreenplayLayout } from "../../src/core/layout/infer-screenplay-layout.js";
import { groupPositionedTextIntoPhysicalLines } from "../../src/core/physical-lines.js";
import { recognizeScreenplay } from "../../src/core/recognition/recognize-screenplay.js";
import type {
  ScreenplayDocument,
  ScreenplayElement,
  StyledText,
} from "../../src/core/screenplay-document.js";
import { extractPositionedPdfText } from "../../src/pdf/extraction/pdf-inspector.js";
import { normalizePhysicalText } from "../../src/pdf/normalization/physical-text.js";
import { brickAndSteelDocument } from "../fixtures/semantic/brick-and-steel-document.js";
import { normalizationGauntletDocument } from "../fixtures/semantic/normalization-gauntlet-document.js";

const brickAndSteelPdfUrl = new URL("../brick-and-steel.pdf", import.meta.url);
const gauntletPdfUrls = [
  new URL("../normalization-gauntlet-clean.pdf", import.meta.url),
  new URL("../normalization-gauntlet-artifacts.pdf", import.meta.url),
];

function styledTextValue(text: StyledText): string {
  return text.runs.map((run) => run.text).join("");
}

function documentTokens(document: ScreenplayDocument): readonly string[] {
  const texts: string[] = [];

  for (const field of document.titlePage) {
    for (const value of field.values) {
      texts.push(styledTextValue(value));
    }
  }

  for (const element of document.elements) {
    if (element.type === "page-break") {
      continue;
    }

    if (element.type === "dual-dialogue") {
      for (const sequence of [element.left, element.right]) {
        texts.push(styledTextValue(sequence.character.text));

        for (const content of sequence.content) {
          texts.push(styledTextValue(content.text));
        }
      }

      continue;
    }

    texts.push(styledTextValue(element.text));
  }

  return texts
    .join(" ")
    .split(/\s+/u)
    .filter((token) => token.length > 0);
}

function isWatermarkElement(element: ScreenplayElement): boolean {
  return (
    (element.type === "action" || element.type === "dialogue") &&
    element.text.runs.map((run) => run.text).join("") === "REVIEW COPY"
  );
}

function recognizePdf(pdfUrl: URL) {
  const positionedText = extractPositionedPdfText(readFileSync(pdfUrl));
  const physicalText = groupPositionedTextIntoPhysicalLines(positionedText);
  const normalizedText = normalizePhysicalText(physicalText);
  const layout = inferScreenplayLayout(normalizedText);

  return recognizeScreenplay(normalizedText, layout);
}

describe("recognizeScreenplay", () => {
  it("recovers the clean screenplay and retains ambiguous body watermarks", () => {
    const results = gauntletPdfUrls.map(recognizePdf);
    const cleanDocument = results[0]!;
    const markedDocument = results[1]!;
    const watermarkElements =
      markedDocument.elements.filter(isWatermarkElement);

    expect(cleanDocument).toEqual(normalizationGauntletDocument);
    expect(watermarkElements).toHaveLength(6);
    expect(markedDocument.titlePage).toEqual(
      normalizationGauntletDocument.titlePage,
    );
    expect(
      documentTokens({
        titlePage: markedDocument.titlePage,
        elements: markedDocument.elements.filter(
          (element) => !isWatermarkElement(element),
        ),
      }),
    ).toEqual(documentTokens(normalizationGauntletDocument));
  });

  it("recovers the complete Brick & Steel screenplay document", () => {
    expect(recognizePdf(brickAndSteelPdfUrl)).toEqual(brickAndSteelDocument);
  });
});
