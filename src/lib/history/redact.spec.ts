import { describe, expect, it } from 'vitest';
import type { QueryParam, RequestUrl } from '../../types/request';
import { buildHistoryEntry, redactedTemplateUrl } from './redact';

function row(key: string, value: string, extra: Partial<QueryParam> = {}): QueryParam {
  return { id: `${key}-${value}`, key, value, enabled: true, hasEquals: value !== '', ...extra };
}

function url(base: string, query: QueryParam[], fragment = ''): RequestUrl {
  return { base, query, fragment };
}

describe('redactedTemplateUrl', () => {
  it('redacts a literal query value', () => {
    expect(redactedTemplateUrl(url('https://api.example.com', [row('token', 'secret')]))).toBe(
      'https://api.example.com?token=<redacted>',
    );
  });

  it('preserves a bare {{variable}} value', () => {
    expect(redactedTemplateUrl(url('https://api.example.com', [row('q', '{{term}}')]))).toBe(
      'https://api.example.com?q={{term}}',
    );
  });

  it('preserves a valueless flag', () => {
    expect(
      redactedTemplateUrl(url('https://api.example.com', [row('flag', '', { hasEquals: false })])),
    ).toBe('https://api.example.com?flag');
  });

  it('keeps a templated base verbatim and redacts mixed rows', () => {
    const result = redactedTemplateUrl(
      url('{{baseUrl}}/search', [row('q', '{{term}}'), row('key', 'abc123')]),
    );
    expect(result).toBe('{{baseUrl}}/search?q={{term}}&key=<redacted>');
  });

  it('handles no query', () => {
    expect(redactedTemplateUrl(url('https://api.example.com/path', []))).toBe(
      'https://api.example.com/path',
    );
  });
});

describe('buildHistoryEntry', () => {
  const meta = { id: 'h1', executedAt: '2026-07-24T00:00:00.000Z' };

  it('records metrics on success and no error kind', () => {
    const entry = buildHistoryEntry(
      {
        method: 'GET',
        url: url('https://x', [row('token', 'secret')]),
        requestId: 'r1',
        result: { status: 200, durationMs: 42, bodyBytes: 128 },
      },
      meta,
    );
    expect(entry).toMatchObject({
      version: 1,
      id: 'h1',
      method: 'GET',
      templateUrl: 'https://x?token=<redacted>',
      status: 200,
      durationMs: 42,
      bodyBytes: 128,
      requestId: 'r1',
      errorKind: null,
    });
  });

  it('records the error kind and null metrics on failure', () => {
    const entry = buildHistoryEntry(
      {
        method: 'POST',
        url: url('https://x', []),
        requestId: null,
        result: { errorKind: 'dns' },
      },
      meta,
    );
    expect(entry.status).toBeNull();
    expect(entry.durationMs).toBeNull();
    expect(entry.bodyBytes).toBeNull();
    expect(entry.errorKind).toBe('dns');
  });
});
