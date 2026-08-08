import type {
  PhysicalTextLine,
  PhysicalTextSpan,
} from "../../core/physical-lines.js";

export interface TextFragment {
  readonly fontFamily: string;
  readonly fontSize: number;
  readonly line: PhysicalTextLine;
  readonly style: string;
  readonly text: string;
  readonly x: number;
  readonly y: number;
}

export interface CharacterCue {
  readonly continued: boolean;
  readonly line: PhysicalTextLine;
  readonly name: string;
}

export const moreMarker = "(MORE)";
export const continuedMarker = "(CONTINUED)";
export const continuedHeaderPattern = /^CONTINUED:\s*\(\d+\)$/;
const continuedCuePattern =
  /^([A-Z][A-Z0-9 .'-]*?)(?: \((?:V\.?O\.?|O\.?S\.?)\))? \((?:CONT['’]D|cont['’]d)\)$/;
const extendedCuePattern =
  /^([A-Z][A-Z0-9 .'-]*?) \((?:V\.?O\.?|O\.?S\.?)\)$/;
const bareCuePattern = /^[A-Z][A-Z0-9 .'-]*$/;

export function createTextFragments(
  line: PhysicalTextLine,
): readonly TextFragment[] {
  const fragments = new Map<string, TextFragment>();

  for (const span of line.spans) {
    const text = line.text.slice(span.start, span.end).trim();

    if (text.length === 0) {
      continue;
    }

    fragments.set(text, createTextFragment(line, span, text));
  }

  const firstSpan = line.spans[0];

  if (firstSpan) {
    const text = line.text.trim();

    if (!fragments.has(text)) {
      fragments.set(text, createTextFragment(line, firstSpan, text));
    }
  }

  return [...fragments.values()];
}

function createTextFragment(
  line: PhysicalTextLine,
  span: PhysicalTextSpan,
  text: string,
): TextFragment {
  return {
    fontFamily: span.font.name.replace(/-\d+$/, "").toLowerCase(),
    fontSize: span.font.size,
    line,
    style: JSON.stringify(span.style),
    text,
    x: span.bounds.x,
    y: span.bounds.y,
  };
}

export function parseCharacterCue(line: PhysicalTextLine): CharacterCue | undefined {
  const text = line.text.trim();
  const continuedMatch = continuedCuePattern.exec(text);

  if (continuedMatch) {
    return { continued: true, line, name: continuedMatch[1]! };
  }

  const extendedMatch = extendedCuePattern.exec(text);

  if (extendedMatch) {
    return { continued: false, line, name: extendedMatch[1]! };
  }

  if (!bareCuePattern.test(text)) {
    return undefined;
  }

  return { continued: false, line, name: text };
}

export function isExplicitPaginationText(text: string): boolean {
  return (
    text === moreMarker ||
    text === continuedMarker ||
    continuedHeaderPattern.test(text) ||
    continuedCuePattern.test(text)
  );
}
