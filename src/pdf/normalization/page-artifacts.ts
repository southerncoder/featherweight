import type { PhysicalText, PhysicalTextLine } from "../../core/physical-lines.js";
import { measurePageExtents } from "../../core/page-extents.js";
import type { NormalizationDiagnostic, SuppressionReason } from "../../core/normalized-text.js";
import { detectRecurringArtifacts } from "./recurring-artifacts.js";
import {
  detectContinuedCharacterCues,
  detectExplicitPaginationMarkers,
} from "./screenplay-pagination.js";
import { detectSourcePageNumbers } from "./source-page-numbers.js";

export interface PageArtifactDecisions {
  readonly diagnostics: ReadonlyMap<PhysicalTextLine, NormalizationDiagnostic>;
  readonly suppressionReasons: ReadonlyMap<
    PhysicalTextLine,
    ReadonlySet<SuppressionReason>
  >;
}

export function detectPageArtifacts(
  physicalText: PhysicalText,
): PageArtifactDecisions {
  const suppressionReasons = new Map<
    PhysicalTextLine,
    Set<SuppressionReason>
  >();
  const diagnostics = new Map<
    PhysicalTextLine,
    NormalizationDiagnostic
  >();

  if (physicalText.lines.length === 0) {
    return { suppressionReasons, diagnostics };
  }

  const geometry = measurePageExtents(physicalText.lines);

  detectRecurringArtifacts(
    physicalText.lines,
    geometry,
    suppressionReasons,
    diagnostics,
  );
  detectSourcePageNumbers(
    physicalText.lines,
    geometry,
    suppressionReasons,
  );
  detectExplicitPaginationMarkers(
    physicalText.lines,
    geometry,
    suppressionReasons,
    diagnostics,
  );
  detectContinuedCharacterCues(
    physicalText.lines,
    geometry,
    suppressionReasons,
    diagnostics,
  );

  return { suppressionReasons, diagnostics };
}
