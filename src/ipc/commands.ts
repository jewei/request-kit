/**
 * The single typed IPC boundary. Nothing else in src/ may import Tauri APIs
 * (bootstrap in main.ts excepted); every backend interaction is a named,
 * typed wrapper here.
 */
import { invoke } from '@tauri-apps/api/core';
import type { SendRequestPayload } from '../types/request';
import type { HttpResponseData } from '../types/response';
import type { RequestFile, WorkspaceBootstrap, WorkspaceNode } from '../types/workspace';
import type { Settings } from '../types/settings';
import type { HistoryEntry } from '../types/history';

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

// --- Storage (M2a) ---

/** Scan `~/.request-kit` and return the workspace bootstrap. */
export function loadWorkspace(): Promise<WorkspaceBootstrap> {
  return invoke<WorkspaceBootstrap>('load_workspace');
}

export function createCollection(name: string): Promise<WorkspaceNode> {
  return invoke<WorkspaceNode>('create_collection', { name });
}

export function createFolder(parentId: string, name: string): Promise<WorkspaceNode> {
  return invoke<WorkspaceNode>('create_folder', { parentId, name });
}

export function createRequest(parentId: string, name: string): Promise<WorkspaceNode> {
  return invoke<WorkspaceNode>('create_request', { parentId, name });
}

export function readRequest(id: string): Promise<RequestFile> {
  return invoke<RequestFile>('read_request', { id });
}

export function writeRequest(id: string, document: RequestFile): Promise<void> {
  return invoke<void>('write_request', { id, document });
}

export function renameNode(id: string, newName: string): Promise<WorkspaceNode> {
  return invoke<WorkspaceNode>('rename_node', { id, newName });
}

export function deleteNode(id: string): Promise<void> {
  return invoke<void>('delete_node', { id });
}

export function duplicateRequest(id: string): Promise<WorkspaceNode> {
  return invoke<WorkspaceNode>('duplicate_request', { id });
}

// --- Settings + history (M2b) ---

export function readSettings(): Promise<Settings> {
  return invoke<Settings>('read_settings');
}

export function writeSettings(settings: Settings): Promise<void> {
  return invoke<void>('write_settings', { settings });
}

export function readHistory(limit: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>('read_history', { limit });
}

export function appendHistory(entry: HistoryEntry): Promise<void> {
  return invoke<void>('append_history', { entry });
}

export function clearHistory(): Promise<void> {
  return invoke<void>('clear_history');
}
