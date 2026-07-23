import type { DraftBody } from '../prepare/types';

/**
 * M1 Content-Type inference (PLAN.md): applied only when the user has not
 * provided a Content-Type header themselves.
 *
 * - explicit user header present → null (never override, never add)
 * - json mode → 'application/json'
 * - raw mode → the body's own contentType, if any
 * - none → null
 */
export function inferContentType(
  body: DraftBody,
  explicitHeader: string | undefined,
): string | null {
  if (explicitHeader !== undefined) {
    return null;
  }
  switch (body.mode) {
    case 'json':
      return 'application/json';
    case 'raw':
      return body.contentType ?? null;
    case 'none':
      return null;
  }
}
