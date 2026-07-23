/** One request-history entry (mirror of the Rust JSONL envelope). Template-first:
 *  `templateUrl` is the pre-substitution URL with literal query values redacted. */
import type { HttpMethod } from './request';

export interface HistoryEntry {
  version: 1;
  id: string;
  /** ISO 8601 timestamp. */
  executedAt: string;
  method: HttpMethod;
  /** Pre-substitution URL; literal query values redacted, `{{var}}` preserved. */
  templateUrl: string;
  status: number | null;
  durationMs: number | null;
  bodyBytes: number | null;
  requestId: string | null;
  errorKind: string | null;
}
