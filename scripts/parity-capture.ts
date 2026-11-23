// Vitest setup file (enabled by PARITY_CAPTURE, see vitest.config.ts): wraps each
// pipeline stage so every call the test suite makes is appended to the capture
// file as one JSON line `{stage, input, output}` (or `error` for a thrown code).
import { appendFileSync } from "node:fs";

import { vi } from "vitest";

const captureFile = process.env.PARITY_CAPTURE!;

function record(stage: string, input: unknown, run: () => unknown): unknown {
  try {
    const output = run();
    appendFileSync(captureFile, `${JSON.stringify({ stage, input, output })}\n`);
    return output;
  } catch (error) {
    const code = (error as { code?: unknown }).code;
    if (typeof code === "string") {
      appendFileSync(captureFile, `${JSON.stringify({ stage, input, error: code })}\n`);
    }
    throw error;
  }
}

vi.mock("../src/core/physical-lines.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/core/physical-lines.js")>();
  return {
    ...actual,
    groupPositionedTextIntoPhysicalLines: (positionedText: Parameters<typeof actual.groupPositionedTextIntoPhysicalLines>[0]) =>
      record("physical", positionedText, () => actual.groupPositionedTextIntoPhysicalLines(positionedText)),
  };
});

vi.mock("../src/pdf/normalization/physical-text.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/pdf/normalization/physical-text.js")>();
  return {
    ...actual,
    normalizePhysicalText: (physicalText: Parameters<typeof actual.normalizePhysicalText>[0], pageIndexes: readonly number[] = []) =>
      record("normalize", { physicalText, pageIndexes }, () => actual.normalizePhysicalText(physicalText, pageIndexes)),
  };
});

vi.mock("../src/core/layout/infer-screenplay-layout.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/core/layout/infer-screenplay-layout.js")>();
  return {
    ...actual,
    inferScreenplayLayout: (normalizedText: Parameters<typeof actual.inferScreenplayLayout>[0]) =>
      record("layout", normalizedText, () => actual.inferScreenplayLayout(normalizedText)),
  };
});

vi.mock("../src/core/recognition/recognize-screenplay.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/core/recognition/recognize-screenplay.js")>();
  return {
    ...actual,
    recognizeScreenplay: (...args: Parameters<typeof actual.recognizeScreenplay>) =>
      record("recognize", { normalizedText: args[0], layout: args[1] }, () => actual.recognizeScreenplay(...args)),
  };
});

vi.mock("../src/fountain/screenplay-document-to-fountain.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/fountain/screenplay-document-to-fountain.js")>();
  return {
    ...actual,
    screenplayDocumentToFountain: (document: Parameters<typeof actual.screenplayDocumentToFountain>[0]) =>
      record("fountain", document, () => actual.screenplayDocumentToFountain(document)),
  };
});

vi.mock("../src/fdx/screenplay-document-to-fdx.js", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/fdx/screenplay-document-to-fdx.js")>();
  return {
    ...actual,
    screenplayDocumentToFDX: (document: Parameters<typeof actual.screenplayDocumentToFDX>[0]) =>
      record("fdx", document, () => actual.screenplayDocumentToFDX(document)),
  };
});
