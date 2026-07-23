/** Canonical URL model — the URL bar is a projection of this (see PLAN.md). */
export interface RequestUrl {
  base: string;
  query: QueryParam[];
  fragment: string;
}

/**
 * Key/value rows store ENCODED substrings verbatim — never decoded, never
 * auto-encoded — so URL round-trips are lossless and `{{templates}}` stay
 * unambiguous.
 */
export interface QueryParam {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
  /** Explicit user choice only; effective sensitivity is computed at use. */
  sensitive?: boolean;
  /** Distinguishes `?flag` from `?flag=`. */
  hasEquals: boolean;
}

export interface KeyValueRow {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
  /** Explicit user choice only; effective sensitivity is computed at use. */
  sensitive?: boolean;
  /** Rows injected by configured auth are always effectively sensitive. */
  origin?: 'user' | 'configuredAuth';
}

export type HttpMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'PATCH'
  | 'DELETE'
  | 'HEAD'
  | 'OPTIONS';

export type BodyMode =
  | 'none'
  | 'raw'
  | 'json'
  | 'formUrlencoded'
  | 'multipart'
  | 'binary'
  | 'graphql';

export type AuthConfig =
  | { type: 'inherit' }
  | { type: 'none' }
  | { type: 'basic'; username: string; password: string }
  | { type: 'bearer'; token: string }
  | { type: 'apikey'; key: string; value: string; in: 'header' | 'query' };

/** Body sent over IPC — everything textual arrives pre-serialized. */
export type SendBody =
  | { mode: 'none' }
  | { mode: 'text'; content: string }
  | { mode: 'multipart'; parts: MultipartPart[] }
  | { mode: 'file'; path: string };

export interface MultipartPart {
  name: string;
  kind: 'text' | 'file';
  value?: string;
  filePath?: string;
  contentType?: string | null;
  fileName?: string | null;
}

/** Payload for the `send_request` command (Rust re-validates all of it). */
export interface SendRequestPayload {
  executionId: string;
  tabId: string;
  method: HttpMethod;
  url: string;
  headers: { name: string; value: string }[];
  body: SendBody;
  timeoutMs: number | null;
  followRedirects: boolean;
  maxBodyBytes: number;
}
