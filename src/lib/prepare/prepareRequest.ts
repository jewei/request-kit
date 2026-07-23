import type {
  HttpMethod,
  KeyValueRow,
  QueryParam,
  SendBody,
} from '../../types/request';
import { inferContentType } from '../http/contentType';
import { serializeRequestUrl, validateBase } from '../url/requestUrl';
import { analyzeScope, buildScope, substitute } from '../variables/resolve';
import type {
  PreparationOptions,
  PreparedRequestResult,
  RequestContext,
  RequestDraft,
  TransportRequest,
  ValidationError,
  Warning,
} from './types';

export type {
  AppDefaults,
  DraftBody,
  PreparationOptions,
  PreparedRequestResult,
  RequestContext,
  RequestDraft,
  TransportRequest,
  ValidationError,
  Warning,
} from './types';

const KNOWN_METHODS: ReadonlySet<string> = new Set<HttpMethod>([
  'GET',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'HEAD',
  'OPTIONS',
]);

/** Case-insensitive header names that make a row effectively sensitive. */
const SENSITIVE_HEADER_NAMES: ReadonlySet<string> = new Set([
  'authorization',
  'proxy-authorization',
  'cookie',
]);

const REDACTED = '<redacted>';

/**
 * Effective sensitivity is computed, never stored (PLAN.md): the explicit user
 * flag, OR a sensitive header name, OR a row injected by configured auth.
 */
function isEffectivelySensitiveHeader(row: KeyValueRow): boolean {
  return (
    row.sensitive === true ||
    SENSITIVE_HEADER_NAMES.has(row.key.toLowerCase()) ||
    row.origin === 'configuredAuth'
  );
}

interface AuthStageState {
  headers: KeyValueRow[];
  query: QueryParam[];
}

/**
 * RESERVED AUTH STAGE — deliberate no-op in M1; activated in M3a.
 * Will apply configured auth (basic/bearer/apikey) to structured headers and
 * `RequestUrl.query` rows here, BEFORE URL serialization, with conflict
 * warnings (manual Authorization / API-key rows win over configured auth).
 */
function applyAuth(state: AuthStageState): AuthStageState {
  return state;
}

function hasControlChars(name: string): boolean {
  for (let i = 0; i < name.length; i++) {
    const code = name.charCodeAt(i);
    if (code < 0x20 || code === 0x7f) {
      return true;
    }
  }
  return false;
}

/**
 * THE canonical request pipeline (PLAN.md). Send, preview, copy-as-cURL, and
 * future codegen all consume its output. Pipeline order: clone draft →
 * buildScope → substitute (resolve mode) / cycle analysis (preserve mode) →
 * reserved auth stage → serialize body → infer Content-Type → serialize the
 * RequestUrl exactly once → validate.
 */
export function prepareRequest(
  draft: RequestDraft,
  context: RequestContext,
  options: PreparationOptions,
): PreparedRequestResult {
  const warnings: Warning[] = [];
  const errors: ValidationError[] = [];

  // 1. Deep-clone — the caller's draft is never mutated.
  const working = structuredClone(draft);

  // 2. Build the variable scope (the pipeline owns scope building).
  const scope = buildScope(context.variableSources);

  // 3. Variable stage.
  const unresolved: string[] = [];
  const seenUnresolved = new Set<string>();
  const applySubstitution = (text: string): string => {
    const result = substitute(text, scope);
    for (const name of result.unresolved) {
      if (!seenUnresolved.has(name)) {
        seenUnresolved.add(name);
        unresolved.push(name);
      }
    }
    return result.output;
  };

  if (options.variableMode === 'preserve') {
    // Substitution skipped; broken variables still surface as warnings.
    const { cycles } = analyzeScope(scope);
    if (cycles.length > 0) {
      warnings.push({
        code: 'variableCycle',
        message: `Variable cycle or depth overflow involving: ${cycles.join(', ')}`,
      });
    }
  } else {
    working.url.base = applySubstitution(working.url.base);
    for (const row of working.url.query) {
      row.key = applySubstitution(row.key);
      row.value = applySubstitution(row.value);
    }
    for (const row of working.headers) {
      row.key = applySubstitution(row.key);
      row.value = applySubstitution(row.value);
    }
    if (working.body.mode === 'raw' || working.body.mode === 'json') {
      working.body.content = applySubstitution(working.body.content);
    }
  }

  // 4. Reserved auth stage (no-op until M3a; must precede URL serialization).
  const authed = applyAuth({ headers: working.headers, query: working.url.query });
  working.headers = authed.headers;
  working.url.query = authed.query;

  // 5. Serialize the selected body.
  const body: SendBody =
    working.body.mode === 'none'
      ? { mode: 'none' }
      : { mode: 'text', content: working.body.content };

  // 6. Infer Content-Type; append only when the user has no enabled
  //    Content-Type header (case-insensitive name match).
  const enabledHeaders = working.headers.filter((row) => row.enabled);
  const explicitContentType = enabledHeaders.find(
    (row) => row.key.toLowerCase() === 'content-type',
  );
  const inferred = inferContentType(
    working.body,
    explicitContentType === undefined ? undefined : explicitContentType.value,
  );

  const redact = options.sensitiveValueMode === 'redact';
  const headers: { name: string; value: string }[] = enabledHeaders.map((row) => ({
    name: row.key,
    value: redact && isEffectivelySensitiveHeader(row) ? REDACTED : row.value,
  }));
  if (inferred !== null) {
    headers.push({ name: 'Content-Type', value: inferred });
  }

  // Redaction operates on the normalized request structure: explicitly
  // sensitive enabled query rows are redacted in the serialized URL too.
  if (redact) {
    for (const row of working.url.query) {
      if (row.enabled && row.sensitive === true) {
        row.value = REDACTED;
      }
    }
  }

  // 7. Serialize the RequestUrl exactly once (base validated first).
  const baseError = validateBase(working.url.base);
  if (baseError !== null) {
    errors.push({ code: 'invalidBase', message: baseError });
  }
  const url = serializeRequestUrl(working.url);

  // 8. Validate the final transport request.
  if (!KNOWN_METHODS.has(working.method)) {
    errors.push({
      code: 'unknownMethod',
      message: `Unknown HTTP method: ${String(working.method)}`,
    });
  }
  if (working.url.base === '') {
    errors.push({ code: 'emptyBase', message: 'The request URL is empty.' });
  }
  for (const row of enabledHeaders) {
    if (row.key === '') {
      errors.push({
        code: 'invalidHeaderName',
        message: 'Header names must not be empty.',
      });
    } else if (hasControlChars(row.key)) {
      errors.push({
        code: 'invalidHeaderName',
        message: `Header name ${JSON.stringify(row.key)} contains control characters.`,
      });
    }
  }
  if (unresolved.length > 0) {
    const message = `Unresolved variables: ${unresolved.join(', ')}`;
    if (options.unresolvedMode === 'error') {
      errors.push({ code: 'unresolvedVariables', message });
    } else {
      warnings.push({ code: 'unresolvedVariables', message });
    }
  }

  if (errors.length > 0) {
    return { ok: false, errors, warnings };
  }

  const request: TransportRequest = {
    method: working.method,
    url,
    headers,
    body,
    timeoutMs: working.settings.timeoutMs ?? context.appDefaults.timeoutMs,
    followRedirects:
      working.settings.followRedirects ?? context.appDefaults.followRedirects,
    maxBodyBytes: context.appDefaults.maxBodyBytes,
  };
  return { ok: true, request, warnings };
}
