/** Mirror of Rust `AppError` (serde camelCase). */
export interface AppError {
  kind: ErrorKind;
  message: string;
  detail?: string;
}

export type ErrorKind =
  | 'invalidUrl'
  | 'cancelled'
  | 'timeout'
  | 'tls'
  | 'dns'
  | 'connection'
  | 'io'
  | 'validation'
  | 'maintenanceInProgress'
  | 'unknown';

const FRIENDLY: Record<ErrorKind, string> = {
  invalidUrl: 'The URL is not valid.',
  cancelled: 'Request cancelled.',
  timeout: 'The request timed out.',
  tls: 'A secure connection could not be established.',
  dns: 'Could not resolve the host. Check the URL or your network.',
  connection: 'Could not connect to the server.',
  io: 'A file or network I/O error occurred.',
  validation: 'The request is invalid.',
  maintenanceInProgress: 'request-kit is busy applying an import. Try again in a moment.',
  unknown: 'Something went wrong.',
};

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'kind' in value &&
    'message' in value
  );
}

export function friendlyMessage(error: AppError): string {
  return FRIENDLY[error.kind] ?? FRIENDLY.unknown;
}
