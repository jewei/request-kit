/**
 * JSON pretty-print + positioned validation (PLAN.md, `lib/format/json.ts`).
 *
 * Error positions are 1-based line/column. V8 (Node/Vitest, WebView2) embeds
 * "(line L column C)" or "at position N" in many SyntaxError messages; when
 * neither is present (e.g. "Unexpected token …" snippet-style messages,
 * "Unexpected end of JSON input") a small scanner locates the first syntax
 * error offset itself.
 */

export type JsonFormatResult =
  | { ok: true; formatted: string }
  | { ok: false; message: string; line: number; column: number };

export type JsonValidateResult =
  | { ok: true }
  | { ok: false; message: string; line: number; column: number };

/** Pretty-print with 2-space indentation. */
export function formatJson(text: string): JsonFormatResult {
  try {
    const parsed: unknown = JSON.parse(text);
    return { ok: true, formatted: JSON.stringify(parsed, null, 2) };
  } catch (error) {
    return failure(error, text);
  }
}

/** Same contract as formatJson, without producing formatted output. */
export function validateJson(text: string): JsonValidateResult {
  try {
    JSON.parse(text);
    return { ok: true };
  } catch (error) {
    return failure(error, text);
  }
}

function failure(
  error: unknown,
  text: string,
): { ok: false; message: string; line: number; column: number } {
  const message = error instanceof Error ? error.message : String(error);
  const { line, column } = errorPosition(message, text);
  return { ok: false, message, line, column };
}

function errorPosition(message: string, text: string): { line: number; column: number } {
  const lineColumn = /line (\d+) column (\d+)/.exec(message);
  if (lineColumn !== null) {
    return { line: Number(lineColumn[1]), column: Number(lineColumn[2]) };
  }
  const position = /position (\d+)/.exec(message);
  if (position !== null) {
    return offsetToLineColumn(text, Number(position[1]));
  }
  return offsetToLineColumn(text, findErrorOffset(text));
}

function offsetToLineColumn(
  text: string,
  offset: number,
): { line: number; column: number } {
  const clamped = Math.min(Math.max(offset, 0), text.length);
  let line = 1;
  let lastNewline = -1;
  for (let i = 0; i < clamped; i++) {
    if (text.charCodeAt(i) === 10) {
      line += 1;
      lastNewline = i;
    }
  }
  return { line, column: clamped - lastNewline };
}

/**
 * Minimal RFC 8259 scanner: returns the offset of the first syntax error.
 * Only used when the engine's message carries no position information, so
 * slight disagreement with the engine's exact recovery point is acceptable.
 */
function findErrorOffset(text: string): number {
  const length = text.length;
  let index = 0;
  let failedAt = -1;

  const fail = (at: number): false => {
    if (failedAt === -1) {
      failedAt = at;
    }
    return false;
  };

  const skipWhitespace = (): void => {
    while (index < length) {
      const c = text.charCodeAt(index);
      if (c === 0x20 || c === 0x09 || c === 0x0a || c === 0x0d) {
        index += 1;
      } else {
        break;
      }
    }
  };

  const parseString = (): boolean => {
    // Caller ensures text[index] === '"'.
    index += 1;
    while (index < length) {
      const c = text.charCodeAt(index);
      if (c === 0x22) {
        index += 1;
        return true;
      }
      if (c < 0x20) {
        return fail(index);
      }
      if (c === 0x5c) {
        // Escape sequence.
        const next = text[index + 1];
        if (next === undefined) {
          return fail(index);
        }
        if ('"\\/bfnrt'.includes(next)) {
          index += 2;
        } else if (next === 'u') {
          const hex = text.slice(index + 2, index + 6);
          if (hex.length === 4 && /^[0-9a-fA-F]{4}$/.test(hex)) {
            index += 6;
          } else {
            return fail(index);
          }
        } else {
          return fail(index);
        }
      } else {
        index += 1;
      }
    }
    return fail(length);
  };

  const parseNumber = (): boolean => {
    const pattern = /-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?/y;
    pattern.lastIndex = index;
    const match = pattern.exec(text);
    if (match === null || match[0].length === 0) {
      return fail(index);
    }
    index += match[0].length;
    return true;
  };

  const parseValue = (): boolean => {
    skipWhitespace();
    if (index >= length) {
      return fail(index);
    }
    const c = text[index];
    if (c === '{') {
      return parseObject();
    }
    if (c === '[') {
      return parseArray();
    }
    if (c === '"') {
      return parseString();
    }
    if (c === '-' || (c >= '0' && c <= '9')) {
      return parseNumber();
    }
    if (text.startsWith('true', index)) {
      index += 4;
      return true;
    }
    if (text.startsWith('false', index)) {
      index += 5;
      return true;
    }
    if (text.startsWith('null', index)) {
      index += 4;
      return true;
    }
    return fail(index);
  };

  const parseObject = (): boolean => {
    index += 1; // past '{'
    skipWhitespace();
    if (text[index] === '}') {
      index += 1;
      return true;
    }
    for (;;) {
      skipWhitespace();
      if (text[index] !== '"') {
        return fail(index >= length ? length : index);
      }
      if (!parseString()) {
        return false;
      }
      skipWhitespace();
      if (text[index] !== ':') {
        return fail(index >= length ? length : index);
      }
      index += 1;
      if (!parseValue()) {
        return false;
      }
      skipWhitespace();
      if (text[index] === ',') {
        index += 1;
        continue;
      }
      if (text[index] === '}') {
        index += 1;
        return true;
      }
      return fail(index >= length ? length : index);
    }
  };

  const parseArray = (): boolean => {
    index += 1; // past '['
    skipWhitespace();
    if (text[index] === ']') {
      index += 1;
      return true;
    }
    for (;;) {
      if (!parseValue()) {
        return false;
      }
      skipWhitespace();
      if (text[index] === ',') {
        index += 1;
        continue;
      }
      if (text[index] === ']') {
        index += 1;
        return true;
      }
      return fail(index >= length ? length : index);
    }
  };

  if (parseValue()) {
    skipWhitespace();
    if (index < length) {
      fail(index);
    }
  }
  return failedAt === -1 ? 0 : failedAt;
}
