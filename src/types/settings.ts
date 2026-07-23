/** App settings (mirror of Rust `settings.json` envelope). */
export interface Settings {
  version: 1;
  theme: 'system' | 'light' | 'dark';
  fontSize: number;
  timeoutMs: number;
  followRedirects: boolean;
  maxBodyBytes: number;
  editorLargeFileKb: number;
}

/** Built-in defaults; also returned by the backend when no file exists. */
export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  theme: 'system',
  fontSize: 13,
  timeoutMs: 30_000,
  followRedirects: true,
  maxBodyBytes: 10 * 1024 * 1024,
  editorLargeFileKb: 1024,
};
