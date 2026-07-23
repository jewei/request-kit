import { describe, expect, it } from 'vitest';
import { formatJson, validateJson } from './json';

describe('formatJson', () => {
  it('pretty-prints with two-space indentation', () => {
    const result = formatJson('{"a":[1,2],"b":{"c":true}}');
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.formatted).toBe('{\n  "a": [\n    1,\n    2\n  ],\n  "b": {\n    "c": true\n  }\n}');
    }
  });

  it('reports a 1-based position for a syntax error', () => {
    const result = formatJson('{\n  "a": 1,\n  "b": oops\n}');
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.line).toBe(3);
      expect(result.column).toBeGreaterThanOrEqual(8);
      expect(result.message).toBeTruthy();
    }
  });

  it('handles unexpected end of input', () => {
    const result = formatJson('{"a": ');
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.line).toBe(1);
      expect(result.column).toBeGreaterThan(0);
    }
  });

  it('handles trailing garbage after a valid value', () => {
    const result = formatJson('{} extra');
    expect(result.ok).toBe(false);
  });

  it('accepts scalars and empty containers', () => {
    expect(formatJson('true').ok).toBe(true);
    expect(formatJson('[]').ok).toBe(true);
    expect(formatJson('"str"').ok).toBe(true);
    expect(formatJson('-1.5e3').ok).toBe(true);
  });
});

describe('validateJson', () => {
  it('mirrors formatJson without output', () => {
    expect(validateJson('{"ok":1}')).toEqual({ ok: true });
    const bad = validateJson('[1,]');
    expect(bad.ok).toBe(false);
  });
});
