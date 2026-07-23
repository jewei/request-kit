/**
 * The single typed IPC boundary. Nothing else in src/ may import Tauri APIs
 * (bootstrap in main.ts excepted); every backend interaction is a named,
 * typed wrapper here.
 */
import { invoke } from '@tauri-apps/api/core';
import type { SendRequestPayload } from '../types/request';
import type { HttpResponseData } from '../types/response';

/** Reveal the main window once the frontend has mounted (window starts hidden). */
export function showMainWindow(): Promise<void> {
  return invoke<void>('show_main_window');
}

/** Returns the resolved storage root (e.g. ~/.request-kit) — also ensures it exists. */
export function storageRoot(): Promise<string> {
  return invoke<string>('storage_root');
}

/**
 * Execute an HTTP request. Resolves with the (possibly truncated) response;
 * rejects with an `AppError`. Cancellation surfaces as kind `cancelled`.
 */
export function sendRequest(payload: SendRequestPayload): Promise<HttpResponseData> {
  return invoke<HttpResponseData>('send_request', { payload });
}

/** Cancel an in-flight request. No-op if the execution already finished. */
export function cancelRequest(executionId: string): Promise<void> {
  return invoke<void>('cancel_request', { executionId });
}

/** Release a retained response body. Idempotent. */
export function releaseResponse(executionId: string): Promise<void> {
  return invoke<void>('release_response', { executionId });
}

/**
 * Rust opens a native save dialog and writes the retained response body.
 * Resolves true if saved, false if the user cancelled the dialog.
 */
export function chooseAndSaveResponse(executionId: string): Promise<boolean> {
  return invoke<boolean>('choose_and_save_response', { executionId });
}
