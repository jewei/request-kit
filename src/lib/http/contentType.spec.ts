import { describe, expect, it } from 'vitest';
import { inferContentType } from './contentType';

describe('inferContentType (M1 rules)', () => {
  it('never adds when the user provided a Content-Type header', () => {
    expect(inferContentType({ mode: 'json', content: '{}' }, 'text/plain')).toBeNull();
    expect(
      inferContentType({ mode: 'raw', content: 'x', contentType: 'text/csv' }, 'a/b'),
    ).toBeNull();
  });

  it('json mode infers application/json', () => {
    expect(inferContentType({ mode: 'json', content: '{}' }, undefined)).toBe(
      'application/json',
    );
  });

  it('raw mode uses the explicit body content type or nothing', () => {
    expect(
      inferContentType({ mode: 'raw', content: 'x', contentType: 'text/csv' }, undefined),
    ).toBe('text/csv');
    expect(inferContentType({ mode: 'raw', content: 'x' }, undefined)).toBeNull();
  });

  it('none mode infers nothing', () => {
    expect(inferContentType({ mode: 'none' }, undefined)).toBeNull();
  });
});
