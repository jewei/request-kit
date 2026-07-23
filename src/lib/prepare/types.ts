import type {
  HttpMethod,
  KeyValueRow,
  RequestUrl,
  SendBody,
} from '../../types/request';
import type { VariableSources } from '../variables/resolve';

/** M1 body modes; the remaining modes (formUrlencoded/multipart/…) land in M4. */
export type DraftBody =
  | { mode: 'none' }
  | { mode: 'raw'; content: string; contentType?: string }
  | { mode: 'json'; content: string };

export interface RequestDraft {
  method: HttpMethod;
  url: RequestUrl;
  headers: KeyValueRow[];
  body: DraftBody;
  /** null = inherit the app default. */
  settings: { timeoutMs: number | null; followRedirects: boolean | null };
}

export interface AppDefaults {
  timeoutMs: number;
  followRedirects: boolean;
  maxBodyBytes: number;
}

export interface RequestContext {
  variableSources: VariableSources;
  appDefaults: AppDefaults;
}

/** One coherent options contract for all pipeline consumers (PLAN.md table). */
export interface PreparationOptions {
  variableMode: 'resolve' | 'preserve';
  sensitiveValueMode: 'include' | 'redact';
  unresolvedMode: 'error' | 'warn';
}

export interface TransportRequest {
  method: HttpMethod;
  url: string;
  headers: { name: string; value: string }[];
  body: SendBody;
  timeoutMs: number | null;
  followRedirects: boolean;
  maxBodyBytes: number;
}

export interface Warning {
  code: string;
  message: string;
}

export interface ValidationError {
  code: string;
  message: string;
}

export type PreparedRequestResult =
  | { ok: true; request: TransportRequest; warnings: Warning[] }
  | { ok: false; errors: ValidationError[]; warnings: Warning[] };
