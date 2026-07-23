/**
 * Single window keydown listener + declarative hotkey registry.
 * 'mod' = Cmd on macOS, Ctrl elsewhere. The registry is exported so future UI
 * (menus, palette, tooltips) can render shortcut hints.
 */
import { getCurrentInstance, onBeforeUnmount } from 'vue';

export interface HotkeyBinding {
  /** e.g. 'mod+enter', 'mod+shift+e' — last part is the key, rest are modifiers. */
  combo: string;
  handler: () => void;
}

const registry: HotkeyBinding[] = [];
let listenerAttached = false;

function isMacPlatform(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /mac|iphone|ipad|ipod/i.test(navigator.platform || navigator.userAgent);
}

function matches(combo: string, event: KeyboardEvent, mac: boolean): boolean {
  const parts = combo.toLowerCase().split('+');
  const key = parts[parts.length - 1];
  const mods = new Set(parts.slice(0, -1));
  const modPressed = mac ? event.metaKey : event.ctrlKey;
  if (mods.has('mod') !== modPressed) return false;
  if (mods.has('shift') !== event.shiftKey) return false;
  if (mods.has('alt') !== event.altKey) return false;
  return event.key.toLowerCase() === key;
}

function onKeydown(event: KeyboardEvent): void {
  const mac = isMacPlatform();
  for (const binding of registry) {
    if (matches(binding.combo, event, mac)) {
      event.preventDefault();
      binding.handler();
      return;
    }
  }
}

/** Live registry (read-only) — feeds shortcut hints in future UI. */
export function hotkeyRegistry(): readonly HotkeyBinding[] {
  return registry;
}

export function useHotkeys() {
  if (!listenerAttached && typeof window !== 'undefined') {
    window.addEventListener('keydown', onKeydown);
    listenerAttached = true;
  }

  const owned: HotkeyBinding[] = [];

  /** Registers a binding; returns an unregister function. */
  function register(combo: string, handler: () => void): () => void {
    const binding: HotkeyBinding = { combo, handler };
    registry.push(binding);
    owned.push(binding);
    return () => {
      const index = registry.indexOf(binding);
      if (index >= 0) registry.splice(index, 1);
    };
  }

  // Bindings registered from a component are removed when it unmounts.
  if (getCurrentInstance()) {
    onBeforeUnmount(() => {
      for (const binding of owned) {
        const index = registry.indexOf(binding);
        if (index >= 0) registry.splice(index, 1);
      }
    });
  }

  return { register, registry: hotkeyRegistry() };
}
