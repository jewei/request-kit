import { describe, expect, it } from 'vitest';
import type { QueryParam, RequestUrl } from '../../types/request';
import {
  emptyRequestUrl,
  parseUrlBar,
  serializeRequestUrl,
  validateBase,
} from './requestUrl';

function row(overrides: Partial<QueryParam> & { id: string }): QueryParam {
  return { key: '', value: '', enabled: true, hasEquals: true, ...overrides };
}

function counterIds(prefix = 'new'): () => string {
  let n = 0;
  return () => `${prefix}-${++n}`;
}

describe('emptyRequestUrl', () => {
  it('returns an empty model', () => {
    expect(emptyRequestUrl()).toEqual({ base: '', query: [], fragment: '' });
  });

  it('returns a fresh object each call', () => {
    const a = emptyRequestUrl();
    const b = emptyRequestUrl();
    expect(a).not.toBe(b);
    expect(a.query).not.toBe(b.query);
  });
});

describe('serializeRequestUrl', () => {
  it('serializes base only', () => {
    expect(serializeRequestUrl({ base: 'https://x.com/a', query: [], fragment: '' })).toBe(
      'https://x.com/a',
    );
  });

  it('joins enabled rows with & and respects hasEquals', () => {
    const url: RequestUrl = {
      base: 'https://x.com/a',
      query: [
        row({ id: '1', key: 'a', value: '1' }),
        row({ id: '2', key: 'flag', hasEquals: false }),
        row({ id: '3', key: 'b', value: '', hasEquals: true }),
      ],
      fragment: '',
    };
    expect(serializeRequestUrl(url)).toBe('https://x.com/a?a=1&flag&b=');
  });

  it('omits disabled rows and skips the ? when no enabled rows exist', () => {
    const url: RequestUrl = {
      base: 'https://x.com/a',
      query: [row({ id: '1', key: 'a', value: '1', enabled: false })],
      fragment: '',
    };
    expect(serializeRequestUrl(url)).toBe('https://x.com/a');
  });

  it('appends the fragment when non-empty', () => {
    expect(
      serializeRequestUrl({
        base: 'https://x.com/a',
        query: [row({ id: '1', key: 'a', value: '1' })],
        fragment: 'sec',
      }),
    ).toBe('https://x.com/a?a=1#sec');
    expect(
      serializeRequestUrl({ base: 'https://x.com/a', query: [], fragment: 'sec' }),
    ).toBe('https://x.com/a#sec');
  });

  it('writes stored values verbatim — never encodes or decodes', () => {
    const url: RequestUrl = {
      base: 'https://x.com/a b',
      query: [
        row({ id: '1', key: 'q', value: 'a%20b+c%ZZ' }),
        row({ id: '2', key: 'tpl', value: '{{searchTerm}}' }),
      ],
      fragment: '',
    };
    expect(serializeRequestUrl(url)).toBe('https://x.com/a b?q=a%20b+c%ZZ&tpl={{searchTerm}}');
  });
});

describe('validateBase', () => {
  it('accepts a normal base', () => {
    expect(validateBase('https://example.com/api/v1')).toBeNull();
    expect(validateBase('{{baseUrl}}/users')).toBeNull();
    expect(validateBase('')).toBeNull();
  });

  it('rejects ? in the resolved base', () => {
    expect(validateBase('https://example.com/api?tenant=123')).toMatch(/\?/);
  });

  it('rejects # in the resolved base', () => {
    expect(validateBase('https://example.com/api#frag')).toBeTruthy();
  });
});

describe('parseUrlBar', () => {
  const empty = emptyRequestUrl();

  function roundTrip(text: string): string {
    return serializeRequestUrl(parseUrlBar(text, empty, counterIds()));
  }

  it('round-trips duplicate keys', () => {
    expect(roundTrip('https://x.com/a?a=1&a=2')).toBe('https://x.com/a?a=1&a=2');
  });

  it('round-trips empty values and flags without equals', () => {
    expect(roundTrip('https://x.com/p?a=')).toBe('https://x.com/p?a=');
    expect(roundTrip('https://x.com/p?flag')).toBe('https://x.com/p?flag');
    expect(roundTrip('https://x.com/p?flag&a=1')).toBe('https://x.com/p?flag&a=1');
  });

  it('round-trips fragments, with and without query', () => {
    expect(roundTrip('https://x.com/p?a=1#sec')).toBe('https://x.com/p?a=1#sec');
    expect(roundTrip('https://x.com/p#frag')).toBe('https://x.com/p#frag');
  });

  it('round-trips %20 vs + vs malformed %ZZ verbatim', () => {
    expect(roundTrip('https://x/p?q=a%20b')).toBe('https://x/p?q=a%20b');
    expect(roundTrip('https://x/p?q=a+b')).toBe('https://x/p?q=a+b');
    expect(roundTrip('https://x/p?q=%ZZ%2')).toBe('https://x/p?q=%ZZ%2');
  });

  it('round-trips template URLs without touching them', () => {
    const text = '{{baseUrl}}/x?q={{q}}';
    const parsed = parseUrlBar(text, empty, counterIds());
    expect(parsed.base).toBe('{{baseUrl}}/x');
    expect(parsed.query).toHaveLength(1);
    expect(parsed.query[0]).toMatchObject({ key: 'q', value: '{{q}}', hasEquals: true });
    expect(serializeRequestUrl(parsed)).toBe(text);
  });

  it('round-trips a bare trailing ? (single empty row)', () => {
    const parsed = parseUrlBar('http://x?', empty, counterIds());
    expect(parsed.query).toHaveLength(1);
    expect(parsed.query[0]).toMatchObject({ key: '', value: '', hasEquals: false });
    expect(serializeRequestUrl(parsed)).toBe('http://x?');
  });

  it('splits rows on the FIRST = only', () => {
    const parsed = parseUrlBar('https://x/p?a=b=c', empty, counterIds());
    expect(parsed.query[0]).toMatchObject({ key: 'a', value: 'b=c', hasEquals: true });
  });

  it('takes the fragment from the first # after the query start', () => {
    const parsed = parseUrlBar('https://x/p?a=1#one#two', empty, counterIds());
    expect(parsed.base).toBe('https://x/p');
    expect(parsed.query[0]).toMatchObject({ key: 'a', value: '1' });
    expect(parsed.fragment).toBe('one#two');
  });

  it('reuses enabled row ids by ordinal position, keeping description/sensitive', () => {
    const prev: RequestUrl = {
      base: 'https://x/p',
      query: [
        row({ id: 'e1', key: 'a', value: '1', description: 'first', sensitive: true }),
        row({ id: 'e2', key: 'b', value: '2', description: 'second' }),
      ],
      fragment: '',
    };
    const parsed = parseUrlBar('https://x/p?x=9&y=8', prev, counterIds());
    expect(parsed.query).toEqual([
      {
        id: 'e1',
        key: 'x',
        value: '9',
        enabled: true,
        hasEquals: true,
        description: 'first',
        sensitive: true,
      },
      { id: 'e2', key: 'y', value: '8', enabled: true, hasEquals: true, description: 'second' },
    ]);
  });

  it('creates deterministic new ids for extra rows and drops unmatched rows', () => {
    const prev: RequestUrl = {
      base: 'https://x/p',
      query: [row({ id: 'e1', key: 'a', value: '1' })],
      fragment: '',
    };
    const grown = parseUrlBar('https://x/p?a=1&b=2&c=3', prev, counterIds('id'));
    expect(grown.query.map((r) => r.id)).toEqual(['e1', 'id-1', 'id-2']);

    const shrunk = parseUrlBar('https://x/p', grown, counterIds());
    expect(shrunk.query).toEqual([]);
  });

  it('is deterministic: same input + same prev + same idgen → same result', () => {
    const prev: RequestUrl = {
      base: 'https://x/p',
      query: [row({ id: 'e1', key: 'a', value: '1' })],
      fragment: '',
    };
    const a = parseUrlBar('https://x/p?a=1&b=2', prev, counterIds('id'));
    const b = parseUrlBar('https://x/p?a=1&b=2', prev, counterIds('id'));
    expect(a).toEqual(b);
  });

  it('keeps disabled rows untouched at their relative positions', () => {
    const prev: RequestUrl = {
      base: 'https://x/p',
      query: [
        row({ id: 'e1', key: 'a', value: '1' }),
        row({ id: 'd1', key: 'off', value: 'v', enabled: false, description: 'kept' }),
        row({ id: 'e2', key: 'b', value: '2' }),
      ],
      fragment: '',
    };
    const parsed = parseUrlBar('https://x/p?x=9&y=8', prev, counterIds());
    expect(parsed.query.map((r) => r.id)).toEqual(['e1', 'd1', 'e2']);
    expect(parsed.query[1]).toEqual({
      id: 'd1',
      key: 'off',
      value: 'v',
      enabled: false,
      hasEquals: true,
      description: 'kept',
    });

    // Fewer enabled rows: e2 dropped, disabled row still in place.
    const fewer = parseUrlBar('https://x/p?x=9', prev, counterIds());
    expect(fewer.query.map((r) => r.id)).toEqual(['e1', 'd1']);

    // More enabled rows: extra row appended after existing positions.
    const more = parseUrlBar('https://x/p?x=9&y=8&z=7', prev, counterIds('id'));
    expect(more.query.map((r) => r.id)).toEqual(['e1', 'd1', 'e2', 'id-1']);
  });

  it('empty text keeps disabled rows and drops enabled rows', () => {
    const prev: RequestUrl = {
      base: 'https://x/p',
      query: [
        row({ id: 'e1', key: 'a', value: '1' }),
        row({ id: 'd1', key: 'off', value: 'v', enabled: false }),
      ],
      fragment: 'frag',
    };
    const parsed = parseUrlBar('', prev, counterIds());
    expect(parsed.base).toBe('');
    expect(parsed.fragment).toBe('');
    expect(parsed.query.map((r) => r.id)).toEqual(['d1']);
  });

  it('never throws on weird input', () => {
    const weird = ['', '?', '#', '??', '?&&&', '?=', '=x', '#?', 'a#f?x', '???a=1', '&', '{{x}}'];
    for (const text of weird) {
      expect(() => parseUrlBar(text, empty, counterIds())).not.toThrow();
    }
  });

  it('does not mutate prev', () => {
    const prev: RequestUrl = {
      base: 'https://x/p',
      query: [row({ id: 'e1', key: 'a', value: '1' })],
      fragment: 'f',
    };
    const snapshot = structuredClone(prev);
    parseUrlBar('https://y/q?b=2', prev, counterIds());
    expect(prev).toEqual(snapshot);
  });

  it('uses crypto.randomUUID when no idgen is provided', () => {
    const parsed = parseUrlBar('https://x/p?a=1&b=2', empty);
    const ids = parsed.query.map((r) => r.id);
    expect(new Set(ids).size).toBe(2);
    for (const id of ids) {
      expect(id).toMatch(/^[0-9a-f-]{36}$/);
    }
  });
});
