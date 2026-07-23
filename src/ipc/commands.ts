/**
 * The single typed IPC boundary. Nothing else in src/ may import Tauri APIs
 * (bootstrap in main.ts excepted); every backend interaction is a named,
 * typed wrapper here.
 */
import { invoke } from '@tauri-apps/api/core';

/** Reveal the main window once the frontend has mounted (window starts hidden). */
export function showMainWindow(): Promise<void> {
  return invoke<void>('show_main_window');
}

/** Returns the resolved storage root (e.g. ~/.request-kit) — M0 smoke check. */
export function storageRoot(): Promise<string> {
  return invoke<string>('storage_root');
}
