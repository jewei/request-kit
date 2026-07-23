import { describe, expect, it } from 'vitest';
import { friendlyMessage, isAppError, type AppError } from './errors';

describe('AppError helpers', () => {
  it('recognizes an AppError shape', () => {
    const err: AppError = { kind: 'dns', message: 'lookup failed' };
    expect(isAppError(err)).toBe(true);
    expect(isAppError(null)).toBe(false);
    expect(isAppError('nope')).toBe(false);
  });

  it('maps every kind to friendly copy', () => {
    const kinds: AppError['kind'][] = [
      'invalidUrl',
      'cancelled',
      'timeout',
      'tls',
      'dns',
      'connection',
      'io',
      'validation',
      'maintenanceInProgress',
      'unknown',
    ];
    for (const kind of kinds) {
      expect(friendlyMessage({ kind, message: 'x' })).toBeTruthy();
    }
  });
});
