# M2b — History, Search, Settings, Themes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Persist settings + a redacted request history, apply themes/font from settings, and let the user search the tree and browse/replay history — all surviving relaunch.

**Architecture:** Two new stateless storage modules (`settings.rs` opaque-Value envelope; `history.rs` append-only JSONL with startup compaction) exposed as commands; frontend settings + history Pinia stores; a pure `redact` lib for template-URL history; SettingsModal + HistoryList + SidebarSearch UI; theme applied to `document.documentElement`.

**Tech Stack:** Rust (serde_json, tempfile, chrono), Vue 3 + Pinia, Vitest.

## Global Constraints

- Files JSON, `"version":1`; `version > 1` ⇒ quarantine + defaults (settings) / skip (history line).
- Perms `0700`/`0600` (existing helpers). History append uses append-mode writer (log durability), not atomic-replace; compaction rewrite is atomic.
- History is **template-first**: literal enabled query values redacted to `<redacted>`, `{{var}}` preserved; no auth headers or resolved secrets ever written. Redaction is a pure frontend function; backend never inspects content.
- `append_history`/`write_settings` rejected during `AppMode != Normal` (`maintenanceInProgress`); reads always allowed.
- Rust keeps settings/history as opaque `serde_json::Value` envelopes; field schemas live in the TS types.
- Search is **by node name** (the tree carries only id/kind/name); url/method search deferred.
- serde camelCase; frontend imports Tauri APIs only in `src/ipc/` + `main.ts`.

---

## File Structure

**Backend:** `storage/settings.rs` (new), `storage/history.rs` (new), `storage/mod.rs` (mod decls), `commands/storage.rs` (add 5 commands + adjust `load_workspace_impl` to read real settings), `lib.rs` (register), `tests/storage_integration.rs` (extend).

**Frontend:** `types/settings.ts`, `types/history.ts` (new); `ipc/commands.ts` (wrappers); `lib/history/redact.ts` (+spec); `stores/settings.ts` (+spec); `stores/history.ts` (+spec); `stores/workspace.ts` (add `filteredTree` +spec); `stores/tabs.ts` (settings-driven defaults + history record; extend spec); `components/settings/SettingsModal.vue`; `components/sidebar/{HistoryList,SidebarSearch}.vue` (+ HistoryList spec); `components/layout/Sidebar.vue` (Collections|History toggle, search, gear); `components/request/RequestSettings.vue` (read settings store); `App.vue` (boot settings+history, applyTheme, `mod+,`).

---

## Task 1: settings.rs

**Files:** Create `src-tauri/src/storage/settings.rs`; modify `storage/mod.rs`.

**Interfaces:**
```rust
pub fn defaults() -> serde_json::Value  // version:1, theme "system", fontSize 13, timeoutMs 30000, followRedirects true, maxBodyBytes 10_485_760, editorLargeFileKb 1024
pub fn read_settings(root: &Path) -> serde_json::Value   // defaults if missing; quarantine+defaults if corrupt/too-new
pub fn write_settings(root: &Path, value: &serde_json::Value) -> Result<(), AppError>  // atomic
pub fn settings_path(root: &Path) -> PathBuf             // root/settings.json
```

- [ ] **Step 1: Failing tests:** missing file → `defaults()`; write then read round-trips a custom theme; a corrupt file → `defaults()` returned + a `.corrupt-*` sibling exists; `version:2` → `defaults()` + quarantined.
- [ ] **Step 2:** `cargo test --lib storage::settings` → FAIL.
- [ ] **Step 3: Implement.** `read_settings`: if `!path.exists()` → `defaults()`. Else read+parse; if parse fails or `version>1` → `quarantine(path, reason)` + `defaults()`. `write_settings` → `ensure_storage_root`-style parent exists (root exists already) then `write_json_atomic`.
- [ ] **Step 4:** re-run → PASS.
- [ ] **Step 5: Commit** `"M2b: settings.rs read/write with default + quarantine"`.

---

## Task 2: history.rs

**Files:** Create `src-tauri/src/storage/history.rs`; modify `storage/mod.rs`.

**Interfaces:**
```rust
pub fn append_history(root: &Path, entry: &serde_json::Value) -> Result<(), AppError>  // validate version==1; append one line
pub fn read_history(root: &Path, limit: usize) -> Vec<serde_json::Value>               // newest-first; compacts >500 at read
pub fn clear_history(root: &Path) -> Result<(), AppError>
pub const HISTORY_COMPACTION_THRESHOLD: usize = 500;
```

- [ ] **Step 1: Failing tests:** append 3 → `read_history(_,10)` returns them newest-first; a blank line and a torn/invalid line between valid ones are skipped; append 600 → after `read_history` the file holds exactly 500 lines (newest) ; `clear_history` empties; `append_history` with `version:2` → `Err(Validation)`.
- [ ] **Step 2:** FAIL.
- [ ] **Step 3: Implement.** `history_dir = root/history`, `file = history_dir/history.jsonl`. Append: validate `entry["version"]==1` else `Err(Validation)`; `ensure_dir(history_dir)`; `OpenOptions::new().create(true).append(true).open(file)`; on Unix set `0600`; write `serde_json::to_string(entry)? + "\n"`. Read: if missing → `vec![]`; read to string, `lines().filter_map(|l| serde_json::from_str(l).ok())` → `Vec<Value>`; if `len > 500` rewrite newest-500 atomically (join with `\n`, `write_json`-style but plain text via a temp file + persist); reverse to newest-first; `truncate(limit)`.
- [ ] **Step 4:** PASS.
- [ ] **Step 5: Commit** `"M2b: history.rs append-only JSONL + startup compaction"`.

---

## Task 3: storage commands + real settings bootstrap + integration

**Files:** Modify `commands/storage.rs`, `lib.rs`, `tests/storage_integration.rs`.

**Interfaces (commands, thin wrappers + `_impl(root, …)`):** `read_settings`, `write_settings(settings)`, `read_history(limit)`, `append_history(entry)`, `clear_history`. `load_workspace_impl` returns `settings::read_settings(&root)` instead of the local default.

- [ ] **Step 1: Failing integration tests** (extend file, driving `settings::`/`history::` + the guard): settings write→read via commands round-trips; `append_history` rejected under `ImportApplying`; `load_workspace` bootstrap `settings` reflects a previously-written theme.
- [ ] **Step 2:** `cargo test --test storage_integration` → FAIL.
- [ ] **Step 3: Implement.** Add `write_settings_impl`/`append_history_impl` with `ensure_normal(state)` guard; read impls without guard. Remove `default_settings()` from `commands/storage.rs` (now `settings::defaults()`); `load_workspace_impl` reads settings. Register 5 commands in `lib.rs`.
- [ ] **Step 4:** `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt` → PASS/clean.
- [ ] **Step 5: Commit** `"M2b: settings + history commands wired + integration"`.

---

## Task 4: frontend types + IPC wrappers

**Files:** Create `src/types/settings.ts`, `src/types/history.ts`; modify `src/ipc/commands.ts`.

```ts
// settings.ts
export interface Settings { version: 1; theme: 'system'|'light'|'dark'; fontSize: number;
  timeoutMs: number; followRedirects: boolean; maxBodyBytes: number; editorLargeFileKb: number }
// history.ts
export interface HistoryEntry { version: 1; id: string; executedAt: string; method: HttpMethod;
  templateUrl: string; status: number|null; durationMs: number|null; bodyBytes: number|null;
  requestId: string|null; errorKind: string|null }
```
IPC: `readSettings(): Promise<Settings>`, `writeSettings(s: Settings): Promise<void>`, `readHistory(limit): Promise<HistoryEntry[]>`, `appendHistory(e: HistoryEntry): Promise<void>`, `clearHistory(): Promise<void>`.

- [ ] **Step 1:** write files. **Step 2:** `bun run typecheck` clean. **Step 3: Commit** `"M2b: settings/history types + IPC wrappers"`.

---

## Task 5: redact lib

**Files:** Create `src/lib/history/redact.ts`, `src/lib/history/redact.spec.ts`.

**Interfaces:**
```ts
export function redactedTemplateUrl(url: RequestUrl): string
export interface HistoryInput { method: HttpMethod; url: RequestUrl; requestId: string|null;
  result: { status: number; durationMs: number; bodyBytes: number } | { errorKind: string } }
export function buildHistoryEntry(input: HistoryInput, meta: { id: string; executedAt: string }): HistoryEntry
```

- [ ] **Step 1: Failing tests:** `?token=secret` → `?token=<redacted>`; `?q={{term}}` preserved; `?flag` (empty value) preserved; mixed literal+template; base with `{{baseUrl}}` kept verbatim; `buildHistoryEntry` success sets status/duration/bytes & null errorKind; error result sets errorKind & null metrics.
- [ ] **Step 2:** FAIL. **Step 3: Implement** — clone url, map query: keep when `value===''` or `/^\{\{[^}]+\}\}$/.test(value)`, else `value='<redacted>'`; `serializeRequestUrl(clone)`. `buildHistoryEntry` per spec. **Step 4:** PASS. **Step 5: Commit** `"M2b: history redaction (template-first) + entry builder"`.

---

## Task 6: settings store

**Files:** Create `src/stores/settings.ts`, `src/stores/settings.spec.ts`.

**Interfaces:** state `settings: Settings`; `load(raw: unknown)` (merge into defaults); `update(patch: Partial<Settings>)` (merge → `applyTheme` → `writeSettings`); `applyTheme()`; getters `timeoutMs/followRedirects/maxBodyBytes/editorLargeFileKb`. Defaults derived from `DEFAULT_SETTINGS` constant.

- [ ] **Step 1: Failing tests** (mock ipc, stub `window.matchMedia`): `applyTheme()` with `theme:'dark'` sets `documentElement.dataset.theme==='dark'` and `--rk-font-size`; `theme:'system'` uses matchMedia result; `update({theme:'light'})` calls `writeSettings` and applies.
- [ ] **Step 2:** FAIL. **Step 3: Implement** (matchMedia guarded for undefined; system listener re-applies). **Step 4:** PASS. **Step 5: Commit** `"M2b: settings store + theme application"`.

---

## Task 7: history store

**Files:** Create `src/stores/history.ts`, `src/stores/history.spec.ts`.

**Interfaces:** state `entries: HistoryEntry[]`; `load(limit=200)`; `record(entry)` (prepend + `appendHistory`); `clear()` (`clearHistory` + empty).

- [ ] **Step 1: Failing tests** (mock ipc): `load` sets entries from `readHistory`; `record` prepends and calls `appendHistory`; `clear` empties and calls `clearHistory`.
- [ ] **Step 2:** FAIL. **Step 3: Implement.** **Step 4:** PASS. **Step 5: Commit** `"M2b: history store"`.

---

## Task 8: workspace filteredTree

**Files:** Modify `src/stores/workspace.ts`, `src/stores/workspace.spec.ts`.

**Interfaces:** `filteredTree(query: string): WorkspaceNode[]` — empty query → full tree; else recursively keep requests whose `name` matches (case-insensitive) and containers on their path (or whose own name matches).

- [ ] **Step 1: Failing tests:** empty query → full tree; query matching a nested request keeps its collection+folder path only; query matching a collection name keeps it with all children; no match → `[]`.
- [ ] **Step 2:** FAIL. **Step 3: Implement** the recursive filter (rebuild containers with filtered children). **Step 4:** PASS. **Step 5: Commit** `"M2b: workspace name search (filteredTree)"`.

---

## Task 9: tabs store — settings defaults + history recording

**Files:** Modify `src/stores/tabs.ts`, `src/stores/tabs.spec.ts`.

**Changes:** `sendActiveTab` reads `useSettingsStore()` for `appDefaults` ({timeoutMs, followRedirects, maxBodyBytes}); on success and on error (not cancel) call `useHistoryStore().record(buildHistoryEntry({method, url: toRaw(tab.draft.url), requestId: tab.requestId, result}, {id: crypto.randomUUID(), executedAt: new Date().toISOString()}))`.

- [ ] **Step 1: Failing tests** (extend spec; mock settings+history stores or their ipc): a successful send records exactly one history entry with the redacted templateUrl and status; an error send records one entry with `errorKind`; the settings store's `timeoutMs` flows into the prepared request (assert via the `sendRequest` payload).
- [ ] **Step 2:** FAIL. **Step 3: Implement.** Import `useSettingsStore`, `useHistoryStore`, `buildHistoryEntry`. Keep `APP_DEFAULTS` as the settings default source. **Step 4:** PASS (all suites). **Step 5: Commit** `"M2b: settings-driven send defaults + history recording"`.

---

## Task 10: SettingsModal

**Files:** Create `src/components/settings/SettingsModal.vue`, `src/components/settings/SettingsModal.spec.ts`; modify `src/components/request/RequestSettings.vue` (read settings store for placeholders).

**Interface:** SettingsModal — teleported modal; controls for theme (System/Light/Dark), fontSize (number), timeoutMs, followRedirects (checkbox), maxBodyBytes (MB number), editorLargeFileKb; each change → `settings.update(patch)`; emits `close`. Live theme/font apply via the store.

- [ ] **Step 1: Failing test:** mount, change the theme select → `settings.update` called with `{theme}` and `documentElement.dataset.theme` updates; Close button emits `close`.
- [ ] **Step 2:** FAIL. **Step 3: Implement** modal + point `RequestSettings` placeholders at `settingsStore` (fallback `APP_DEFAULTS`). **Step 4:** PASS + typecheck/lint. **Step 5: Commit** `"M2b: SettingsModal + settings-driven request defaults"`.

---

## Task 11: Sidebar History/Search + HistoryList + SidebarSearch

**Files:** Create `src/components/sidebar/HistoryList.vue`, `src/components/sidebar/SidebarSearch.vue`, `src/components/sidebar/HistoryList.spec.ts`; modify `src/components/layout/Sidebar.vue`.

**Interfaces:** `SidebarSearch` — `v-model`-style `query` (emits `update:query`). `HistoryList` — props `entries`; emits `replay(entry)`, `clear`; renders method + `templateUrl` + status/errorKind + relative time. `Sidebar` — a Collections|History segmented toggle; in Collections mode shows SidebarSearch + `workspace.filteredTree(query)`; in History mode shows HistoryList wired to the history store; gear button opens SettingsModal; `replay` → open the request (`requestId` present & found) else open a scratch tab pre-filled with method+URL.

- [ ] **Step 1: Failing test:** mount HistoryList with two entries (one with `?token=<redacted>`, one error) → renders both template URLs, shows the error kind, no secret text; clicking a row emits `replay` with the entry; Clear emits `clear`.
- [ ] **Step 2:** FAIL. **Step 3: Implement** components + Sidebar mode toggle/search/gear/replay. `replay` for a scratch fallback parses the templateUrl into the draft via `parseUrlBar`. **Step 4:** PASS + typecheck/lint. **Step 5: Commit** `"M2b: sidebar history list, search box, mode toggle"`.

---

## Task 12: Boot wiring + smoke + verify + push

**Files:** Modify `src/App.vue` (boot: after `loadWorkspace`, `settings.load(bootstrap.settings)` + `applyTheme()`; `history.load()`; register `mod+,` → open SettingsModal via a shared UI flag or emit); modify `docs/smoke.md`.

- [ ] **Step 1:** Wire boot. Settings modal visibility: a `ui`-level ref in App (or a tiny `stores/ui.ts` `settingsOpen`), toggled by the gear button (Sidebar) and `mod+,`. Simplest: `stores/ui.ts` with `settingsOpen` + `toggleSettings`; Sidebar gear and App hotkey both call it; App renders `<SettingsModal v-if="ui.settingsOpen" @close="ui.settingsOpen=false"/>`.
- [ ] **Step 2:** Rewrite `docs/smoke.md` M2b section: redacted history (`?token=<redacted>`, no auth header); theme persists across relaunch; window geometry restored + off-screen clamp; changed default timeout used by a new send; tree search filters; clear history empties.
- [ ] **Step 3: Full verify:** `bun run test && bun run typecheck && bun run lint && bun run build`; `cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`.
- [ ] **Step 4: Commit + push** `"M2b complete: history, search, settings, themes"`; confirm CI.

---

## Self-review notes

- **Spec coverage:** settings persistence (T1,T3,T6,T10), history JSONL + compaction (T2,T3,T7), redaction (T5), search (T8), history recording on send (T9), themes (T6,T10,T12), SettingsModal (T10), history list + search UI (T11), boot + window-state smoke (T12). Backup mgmt deferred (M3b); window-state code already exists (smoke only).
- **Type consistency:** `Settings`/`HistoryEntry` identical Rust-envelope ↔ TS; command names match across `commands/storage.rs`, `lib.rs`, `ipc/commands.ts`.
- **Deviation:** tree search is name-only (tree lacks url/method); noted in Global Constraints.
