import type { QueryParam, RequestUrl } from '../../types/request';

/**
 * RequestUrl model — the URL bar is an editable projection of this (PLAN.md,
 * "URL model"). Keys and values store ENCODED substrings verbatim: nothing in
 * this module ever percent-encodes or -decodes, so round-trips are lossless and
 * templates like `{{baseUrl}}/x` survive. `new URL()` is deliberately never
 * used — string splitting only.
 */

export function emptyRequestUrl(): RequestUrl {
  return { base: '', query: [], fragment: '' };
}

/**
 * Serialize the canonical model to the displayed/transport URL:
 * base + '?' + enabled rows ('&'-joined, `key` or `key=value` per `hasEquals`)
 * + '#' + fragment. Disabled rows never appear. Values written verbatim.
 */
export function serializeRequestUrl(url: RequestUrl): string {
  const enabled = url.query.filter((row) => row.enabled);
  let out = url.base;
  if (enabled.length > 0) {
    out +=
      '?' +
      enabled
        .map((row) => row.key + (row.hasEquals ? '=' + row.value : ''))
        .join('&');
  }
  if (url.fragment !== '') {
    out += '#' + url.fragment;
  }
  return out;
}

/**
 * The (post-substitution) base must not introduce `?` or `#` — query values
 * belong in parameter rows; this prevents double-`?` serialization.
 * Returns an error message, or null when valid.
 */
export function validateBase(resolvedBase: string): string | null {
  if (resolvedBase.includes('?') || resolvedBase.includes('#')) {
    return "The URL base must not contain '?' or '#'. Move query values into parameter rows.";
  }
  return null;
}

const defaultMakeId = (): string => crypto.randomUUID();

interface ParsedPart {
  key: string;
  value: string;
  hasEquals: boolean;
}

/**
 * Deterministic URL-bar parse-back + reconciliation (PLAN.md):
 * 1. parse enabled rows from the bar (split on first '?', '#' after the query
 *    start, '&' between rows, FIRST '=' inside a row — never decoded);
 * 2. reuse prev's enabled-row ids by ordinal position (keeping their
 *    `description`/`sensitive`);
 * 3. update reused rows' key/value/hasEquals;
 * 4. create new ids for extra parsed rows (via `makeId`);
 * 5. drop unmatched prev enabled rows;
 * 6. keep disabled rows untouched at their existing relative positions.
 * Never throws; empty text yields base '' and keeps only disabled rows.
 */
export function parseUrlBar(
  text: string,
  prev: RequestUrl,
  makeId: () => string = defaultMakeId,
): RequestUrl {
  let base: string;
  let queryText: string | null = null;
  let fragment = '';

  const questionIdx = text.indexOf('?');
  if (questionIdx === -1) {
    const hashIdx = text.indexOf('#');
    if (hashIdx === -1) {
      base = text;
    } else {
      base = text.slice(0, hashIdx);
      fragment = text.slice(hashIdx + 1);
    }
  } else {
    base = text.slice(0, questionIdx);
    const rest = text.slice(questionIdx + 1);
    const hashIdx = rest.indexOf('#');
    if (hashIdx === -1) {
      queryText = rest;
    } else {
      queryText = rest.slice(0, hashIdx);
      fragment = rest.slice(hashIdx + 1);
    }
  }

  const parsedParts: ParsedPart[] =
    queryText === null
      ? []
      : queryText.split('&').map((part) => {
          const eq = part.indexOf('=');
          return eq === -1
            ? { key: part, value: '', hasEquals: false }
            : { key: part.slice(0, eq), value: part.slice(eq + 1), hasEquals: true };
        });

  const prevEnabled = prev.query.filter((row) => row.enabled);
  const reconciled = parsedParts.map((part, index) => {
    const reused = prevEnabled[index];
    const row: QueryParam = {
      id: reused ? reused.id : makeId(),
      key: part.key,
      value: part.value,
      enabled: true,
      hasEquals: part.hasEquals,
    };
    if (reused !== undefined && reused.description !== undefined) {
      row.description = reused.description;
    }
    if (reused !== undefined && reused.sensitive !== undefined) {
      row.sensitive = reused.sensitive;
    }
    return row;
  });

  // Walk prev.query, replacing the sequence of enabled rows with the
  // reconciled list while leaving disabled rows in place at their indices.
  const query: QueryParam[] = [];
  let next = 0;
  for (const row of prev.query) {
    if (row.enabled) {
      if (next < reconciled.length) {
        query.push(reconciled[next]);
        next += 1;
      }
      // else: unmatched prev enabled row — dropped.
    } else {
      query.push({ ...row });
    }
  }
  while (next < reconciled.length) {
    query.push(reconciled[next]);
    next += 1;
  }

  return { base, query, fragment };
}
