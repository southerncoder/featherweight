import type { PhysicalTextLine } from "../../core/physical-lines.js";
import type { NormalizationDiagnostic, SuppressionReason } from "../../core/normalized-text.js";

export function addSuppressionReason(
  suppressionReasons: Map<PhysicalTextLine, Set<SuppressionReason>>,
  line: PhysicalTextLine,
  reason: SuppressionReason,
): void {
  const reasons = suppressionReasons.get(line);

  if (reasons) {
    reasons.add(reason);
    return;
  }

  suppressionReasons.set(line, new Set([reason]));
}

export function addPaginationDiagnostic(
  diagnostics: Map<PhysicalTextLine, NormalizationDiagnostic>,
  line: PhysicalTextLine,
): void {
  addArtifactDiagnostic(diagnostics, line, "screenplay-pagination");
}

export function addArtifactDiagnostic(
  diagnostics: Map<PhysicalTextLine, NormalizationDiagnostic>,
  line: PhysicalTextLine,
  candidateReason: SuppressionReason,
): void {
  diagnostics.set(line, {
    code: "AMBIGUOUS_PAGE_ARTIFACT",
    pageIndex: line.pageIndex,
    sourceIndexes: line.spans.map((span) => span.sourceIndex),
    candidateReasons: [candidateReason],
  });
}
