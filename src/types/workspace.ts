/** Workspace tree + saved-request types (mirror of Rust `storage` serde types). */
import type { DraftBody } from '../lib/prepare/types';
import type { HttpMethod, KeyValueRow, RequestUrl } from './request';

export type NodeKind = 'collection' | 'folder' | 'request';

/** One node in the sidebar tree. Requests omit `children`. */
export interface WorkspaceNode {
  id: string;
  kind: NodeKind;
  name: string;
  children?: WorkspaceNode[];
}

/**
 * On-disk request document. Rust round-trips this verbatim (it only parses the
 * `version`/`id`/`name` envelope). `auth`/`variables` are persisted with M2a
 * defaults so M3a/M4 need no migration.
 */
export interface RequestFile {
  version: 1;
  id: string;
  name: string;
  method: HttpMethod;
  url: RequestUrl;
  headers: KeyValueRow[];
  body: DraftBody;
  auth: { type: string };
  variables: unknown[];
  settings: { timeoutMs: number | null; followRedirects: boolean | null };
}

/** A file moved aside during a scan (corrupt or duplicate id). */
export interface QuarantineReport {
  original: string;
  reason: string;
}

/**
 * One `load_workspace` bootstrap. In M2a only `tree` and `quarantined` carry
 * real data; the rest are defaults filled in by M2b.
 */
export interface WorkspaceBootstrap {
  tree: WorkspaceNode[];
  environments: unknown[];
  globals: unknown[];
  settings: unknown;
  uiState: unknown;
  quarantined: QuarantineReport[];
}
