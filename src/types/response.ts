/** Mirror of Rust `HttpResponseData` (serde camelCase). */
export interface HttpResponseData {
  executionId: string;
  status: number;
  /** Canonical label for the status code, not the server's reason phrase. */
  statusText: string;
  httpVersion: string;
  /** Duplicate values preserved; global ordering unspecified (HeaderMap). */
  headers: [string, string][];
  durationMs: number;
  /** Decoded body bytes (after automatic gzip/brotli/deflate decoding). */
  bodyBytes: number;
  contentType: string | null;
  finalUrl: string;
  /** UTF-8 text, capped at the IPC display limit; null when binary. */
  body: string | null;
  bodyTruncated: boolean;
  isBinary: boolean;
  downloadCapped: boolean;
}
