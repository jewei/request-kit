/**
 * Template-first history redaction. The stored history URL is the draft URL
 * *before* variable substitution, with every literal enabled query value
 * replaced by `<redacted>` and `{{var}}` placeholders preserved. This is the
 * single place secrets are stripped before they can reach disk (PLAN.md).
 */
import type { HttpMethod, RequestUrl } from '../../types/request';
import type { HistoryEntry } from '../../types/history';
import { serializeRequestUrl } from '../url/requestUrl';

const REDACTED = '<redacted>';
const TEMPLATE_ONLY = /^\{\{[^}]+\}\}$/;

/** A value is kept only when it is empty or a bare `{{variable}}`; anything
 *  else is a literal that may carry a secret and is redacted. */
function redactValue(value: string): string {
  if (value === '' || TEMPLATE_ONLY.test(value)) return value;
  return REDACTED;
}

export function redactedTemplateUrl(url: RequestUrl): string {
  const redacted: RequestUrl = {
    base: url.base,
    fragment: url.fragment,
    query: url.query.map((row) => ({ ...row, value: redactValue(row.value) })),
  };
  return serializeRequestUrl(redacted);
}

export type HistoryResult =
  | { status: number; durationMs: number; bodyBytes: number }
  | { errorKind: string };

export interface HistoryInput {
  method: HttpMethod;
  url: RequestUrl;
  requestId: string | null;
  result: HistoryResult;
}

/** Assemble a history entry. `meta` (id + timestamp) is injected so this stays
 *  pure and testable. */
export function buildHistoryEntry(
  input: HistoryInput,
  meta: { id: string; executedAt: string },
): HistoryEntry {
  const base = {
    version: 1 as const,
    id: meta.id,
    executedAt: meta.executedAt,
    method: input.method,
    templateUrl: redactedTemplateUrl(input.url),
    requestId: input.requestId,
  };
  const { result } = input;
  if ('errorKind' in result) {
    return { ...base, status: null, durationMs: null, bodyBytes: null, errorKind: result.errorKind };
  }
  return {
    ...base,
    status: result.status,
    durationMs: result.durationMs,
    bodyBytes: result.bodyBytes,
    errorKind: null,
  };
}
