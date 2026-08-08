export function isUppercaseText(text: string): boolean {
  const letters = text.match(/\p{L}/gu) ?? [];

  return (
    letters.length >= 2 &&
    letters.every((letter) => letter === letter.toLocaleUpperCase())
  );
}

export function isCueUppercaseText(text: string): boolean {
  return isUppercaseText(removeTrailingParenthesizedGroups(text));
}

function removeTrailingParenthesizedGroups(text: string): string {
  let prefixEnd = text.trimEnd().length;

  while (prefixEnd > 0 && text[prefixEnd - 1] === ")") {
    let depth = 0;
    let openingIndex = -1;

    for (let index = prefixEnd - 1; index >= 0; index -= 1) {
      if (text[index] === ")") {
        depth += 1;
      } else if (text[index] === "(") {
        depth -= 1;

        if (depth === 0) {
          openingIndex = index;
          break;
        }
      }
    }

    if (openingIndex < 0) {
      break;
    }

    prefixEnd = text.slice(0, openingIndex).trimEnd().length;
  }

  return text.slice(0, prefixEnd);
}

export function isParenthetical(text: string): boolean {
  const trimmedText = text.trim();

  return trimmedText.startsWith("(") && trimmedText.endsWith(")");
}

export function isParentheticalFragment(text: string): boolean {
  const trimmedText = text.trim();

  return trimmedText.startsWith("(") || trimmedText.endsWith(")");
}

export function isCueParentheticalFragment(text: string): boolean {
  const trimmedText = text.trim();

  return (
    trimmedText.startsWith("(") ||
    (trimmedText.endsWith(")") && !trimmedText.includes("("))
  );
}
