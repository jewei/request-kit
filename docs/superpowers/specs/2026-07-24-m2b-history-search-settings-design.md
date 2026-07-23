# M2b — History, search, settings, themes, window-state (design)

Milestone M2b from `PLAN.md`. Continues the storage foundation from M2a. This
spec fixes the open decisions and module boundaries; the plan is derived from it.

**Exit (PLAN.md):** history shows template URLs only; theme and window state
survive relaunch.

## Decisions (autonomous — recommended options, per delegated authority)

1. **Backup management UI is deferred to M3b.** Backups are only created by the
   import flow (M3b); there is nothing to manage in M2b. The SettingsModal ships
   theme/font/request-default settings now; a "Backups" section is added when
   backups exist. (Deviation from PLAN's "incl. backup management" — justified:
   the feature has no data until M3b.)
2. **Window-state is smoke-only.** `tauri-plugin-window-state` (registered in M0
   with `StateFlags::all() − VISIBLE`) already restores geometry. M2b adds no
   code here; it verifies restore + off-screen clamping in the smoke checklist.
3. **History redaction is a pure frontend function; the backend persists.** The
   frontend owns the draft `RequestUrl` and computes the redacted template URL
   (serialize the pre-substitution URL, redact every literal enabled query
   value, preserve `{{var}}`), then calls `append_history(entry)`. The backend
   only appends/reads/compacts JSONL and never inspects request content.
4. **Settings drive send defaults.** The hardcoded `APP_DEFAULTS` becomes the
   settings store's initial values; `tabs.sendActiveTab` and `RequestSettings`
   read the live settings. `APP_DEFAULTS` stays as the fallback constant.
5. **Search scope = collections tree** (name + method + templateUrl of the
   request's saved URL), in-memory, in the workspace store. History has its own
   list; a history filter is a small text match over `templateUrl`/`method`.
6. **`ui-state.json` stays stubbed.** openTabs/activeTabId (M3a), activeEnvId
   (M3a), sidebar toggle/expanded (M3b hotkeys) are out of M2b. Theme + numeric
   prefs live in `settings.json`. M2b persists `settings.json` and
   `history.jsonl` only.
7. **SettingsModal opens** from a gear button in the sidebar header and the
   `mod+,` hotkey.

## On-disk additions (M2a layout unchanged)

```
~/.request-kit/
├── settings.json     # { version:1, theme, fontSize, timeoutMs, followRedirects,
│                     #   maxBodyBytes, editorLargeFileKb }
└── history/
    └── history.jsonl  # append-only; one JSON object per line
```

Both created lazily on first write. `0700`/`0600` perms via the existing
`ensure_dir`/`write_json_atomic` helpers (history append uses a dedicated
append writer, see below).

## Backend (`src-tauri/src/storage/`)

- `settings.rs` — `read_settings(root) -> Value` (returns defaults if the file
  is missing; quarantines + returns defaults if corrupt/too-new),
  `write_settings(root, Value) -> ()` (atomic via `write_json_atomic`). Rust
  keeps settings as an opaque `Value` envelope with a `version` check — the
  field schema lives in the TS `Settings` type (same pattern as `RequestFile`).
- `history.rs`:
  - `append_history(root, entry: Value)` — validate `version==1`; append one
    compact JSON line to `history/history.jsonl` (create dir/file with perms;
    open in append mode; write `line + "\n"`; not atomic-replace — append is the
    correct durability model for a log). A torn last line is tolerated on read.
  - `read_history(root, limit) -> Vec<Value>` — read lines, skip blank/torn
    lines, parse each; **startup compaction:** if the file holds more than 500
    valid entries, rewrite it (atomically) to the newest 500 before returning;
    return the newest `limit` (most-recent-first).
  - `clear_history(root)` — truncate/remove the file.
- `commands/storage.rs` — add `read_settings`, `write_settings`, `read_history`,
  `append_history`, `clear_history` (thin wrapper + `_impl(&AppState-less, root)`;
  these are stateless disk ops, so `_impl` takes `root: &Path` directly and the
  command resolves `ensure_storage_root()`). `append_history`/`write_settings`
  respect the maintenance guard (rejected during import apply/recovery);
  `read_*` are always allowed.

`WorkspaceBootstrap` now returns the real `settings` (from `read_settings`) and
keeps `history` out of the bootstrap (loaded lazily by the history store).

## Frontend

**Types (`src/types/`):**
- `settings.ts` — `Settings { version:1; theme:'system'|'light'|'dark';
  fontSize:number; timeoutMs:number; followRedirects:boolean; maxBodyBytes:number;
  editorLargeFileKb:number }`.
- `history.ts` — `HistoryEntry { version:1; id; executedAt:string (ISO);
  method; templateUrl; status:number|null; durationMs:number|null;
  bodyBytes:number|null; requestId:string|null; errorKind:string|null }`.

**Pure lib (`src/lib/history/redact.ts`):**
- `redactedTemplateUrl(url: RequestUrl): string` — serialize the pre-substitution
  URL with every **literal** enabled query value replaced by `<redacted>` and
  `{{var}}` placeholders preserved (a value is a bare `{{name}}` ⇒ keep; else
  redact). Reuses `serializeRequestUrl`-style joining but never resolves vars.
- `buildHistoryEntry(input): HistoryEntry` — assembles the entry from a send's
  method + draft url + result (status/duration/bytes or errorKind) + requestId +
  an injected `executedAt`/`id` (so it stays pure/testable).

**Stores:**
- `stores/settings.ts` — `settings` ref (initialized from `APP_DEFAULTS`-derived
  defaults), `load()` (from bootstrap's `settings`), `update(patch)` (merge +
  debounced `writeSettings`), and `applyTheme()` (sets `document.documentElement`
  `data-theme` = effective theme and `--rk-font-size`; a `matchMedia` listener
  re-applies when `theme==='system'`). Getters: `timeoutMs`, `followRedirects`,
  `maxBodyBytes`, `editorLargeFileKb`.
- `stores/history.ts` — `entries` ref, `load(limit=200)`, `record(entry)`
  (optimistic prepend + `appendHistory`), `clear()`.

**Tabs store wiring:** `sendActiveTab` reads `settingsStore` for `appDefaults`;
on completion (success or error) it calls `historyStore.record(buildHistoryEntry(
  { method, url: tab.draft.url, requestId, result }, { id: uuid, executedAt: new Date().toISOString() }))`.
Cancelled sends record nothing (retain-nothing rule).

**Components:**
- `settings/SettingsModal.vue` — teleported modal; theme select (System/Light/
  Dark), font size, default timeout, follow redirects, max body (MB), editor
  large-file threshold (KB). Live-applies theme/font; persists via
  `settings.update`.
- `sidebar/HistoryList.vue` — most-recent-first list of `templateUrl` + method +
  status/error + relative time; click re-opens the request when `requestId` is
  set and still exists (else opens a scratch tab pre-filled with method + URL);
  a "Clear history" action (confirm).
- `sidebar/SidebarSearch.vue` — a search input; drives a `query` the Sidebar
  passes to the workspace store's filter.
- `layout/Sidebar.vue` — add a **Collections | History** segment toggle, the
  search box (Collections mode), a gear button opening SettingsModal, and render
  `HistoryList` in History mode. The tree renders the filtered nodes.
- `stores/workspace.ts` — add `filteredTree(query)`: recursively keep requests
  whose name/method/url matches and the containers on their path; empty query =
  full tree.

**Theme + settings boot (`App.vue`/`main.ts`):** after `loadWorkspace`, the
settings store loads from the bootstrap and calls `applyTheme()`; the history
store loads lazily when History mode is first opened (or on boot — cheap).
Register `mod+,` to open SettingsModal.

## Error handling

- Missing `settings.json` / `history.jsonl` ⇒ defaults / empty (never an error).
- Corrupt/too-new `settings.json` ⇒ quarantined, defaults returned, reported via
  the existing quarantine notice.
- Torn last JSONL line tolerated; unparseable lines skipped on read.
- `append_history`/`write_settings` rejected during import apply/recovery
  (`maintenanceInProgress`).

## Testing

**Rust (`cargo test`, tempdirs):** settings round-trip + default-on-missing +
quarantine-on-corrupt; history append→read order (most-recent-first); blank/torn
line tolerance; startup compaction to newest 500; clear; maintenance guard on
`append_history`/`write_settings`.

**TS (Vitest):** `redactedTemplateUrl` (literal query value redacted, `{{var}}`
preserved, mixed, no-query, fragment); `buildHistoryEntry` (success vs error
shapes); settings store `applyTheme` sets `data-theme`/`--rk-font-size` and
tracks system; history store record prepends + calls `appendHistory`; workspace
`filteredTree` (match by name/method/url, keeps ancestor containers, empty query
= full); tabs `sendActiveTab` records one history entry on success and on error,
none on cancel; SettingsModal emits updates; HistoryList renders template URLs
only (no secrets) and re-opens on click.

**Manual smoke (`docs/smoke.md` M2b):** send a request with a `?token=secret` →
history shows `?token=<redacted>` (template only, no auth header); switch theme →
persists across relaunch; window position/size restored on relaunch (and clamps
if moved off-screen); change default timeout in Settings → a new request uses it;
search filters the tree; clear history empties the list.

## Milestone boundary

M2b = history + search + settings/themes + window-state verification. Multi-tab,
environments, variables, auth (M3a); cURL/import/export, full hotkey set, backup
management (M3b); everything else later. `ui-state.json` persistence beyond
defaults is out.
