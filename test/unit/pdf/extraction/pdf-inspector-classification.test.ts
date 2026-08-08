import { afterEach, describe, expect, it, vi } from "vitest";

const nativeBoundary = vi.hoisted(() => ({
  classification: { pageCount: 0, pagesNeedingOcr: [] } as unknown,
}));

vi.mock("node:module", async (importOriginal) => {
  const nodeModule = await importOriginal<typeof import("node:module")>();

  return {
    ...nodeModule,
    createRequire: () => () => ({
      classifyPdf: () => nativeBoundary.classification,
    }),
  };
});

afterEach(() => {
  nativeBoundary.classification = { pageCount: 0, pagesNeedingOcr: [] };
  vi.resetModules();
});

describe("classifyPdfForOcr native result validation", () => {
  it.each([
    ["a non-object result", null],
    ["a negative page count", { pageCount: -1, pagesNeedingOcr: [] }],
    ["a fractional page count", { pageCount: 1.5, pagesNeedingOcr: [] }],
    ["non-array OCR pages", { pageCount: 1, pagesNeedingOcr: null }],
    ["a fractional OCR page", { pageCount: 2, pagesNeedingOcr: [0.5] }],
    ["a negative OCR page", { pageCount: 2, pagesNeedingOcr: [-1] }],
    ["an out-of-range OCR page", { pageCount: 2, pagesNeedingOcr: [2] }],
    ["duplicate OCR pages", { pageCount: 2, pagesNeedingOcr: [1, 1] }],
  ])("rejects %s atomically", async (_name, classification) => {
    nativeBoundary.classification = classification;
    const [{ classifyPdfForOcr }, { PdfInspectionError }] = await Promise.all([
      import("../../../../src/pdf/extraction/pdf-inspector.js"),
      import("../../../../src/pdf/inspection-error.js"),
    ]);
    let thrown: unknown;

    try {
      classifyPdfForOcr(new Uint8Array([0x25, 0x50, 0x44, 0x46]));
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(PdfInspectionError);
    expect(thrown).toMatchObject({
      code: "PDF_INSPECTION_FAILED",
      message: "PDF inspection failed.",
      cause: expect.any(Error),
    });
  });

  it("accepts a zero-page classification", async () => {
    const { classifyPdfForOcr } = await import(
      "../../../../src/pdf/extraction/pdf-inspector.js"
    );

    expect(
      classifyPdfForOcr(new Uint8Array([0x25, 0x50, 0x44, 0x46])),
    ).toEqual({ pageCount: 0, pagesNeedingOcr: [] });
  });
});
