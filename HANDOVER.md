# Handover — request-kit (M1 in progress)

Read `PLAN.md` first (revision 3.4 — the approved, self-contained spec; MVP.md is the
feature scope). This file records exactly where implementation stands and what remains.

## State

**M0 (Foundation): complete, verified, pushed.** Tauri 2.11 + Vue 3.5 + Vite 8 + Bun
scaffold; single-instance first; window-state with hidden-until-ready; minimal webview
capabilities (`core:default` + `clipboard-manager:allow-write-text`); `~/.request-kit`
created 0700 on first run; CI matrix (`.github/workflows/build.yml`) pushed — verify
`gh run list` for installer builds; unsigned-installer launch on Windows still needs a
human (M0 exit condition, tracked in `docs/smoke.md`).

**M1 (Send one request): ~75% done.** Three layers were built in parallel:

### Done and verified

- **Rust backend — complete.** `src-tauri/src/http/{types,clients,validate,error_map,retain,executor}.rs`,
  commands in `src-tauri/src/commands/http.rs` (`send_request`, `cancel_request`,
  `release_response`, `choose_and_save_response` with Rust-owned save dialog), wired in
  `lib.rs`/`state.rs`. **20 tests green** (`cargo test`), including the in-process
  TcpListener fixture matrix: json/redirects/gzip/dup-headers/cap/binary/cancel/abrupt-close/
  timeout/maintenance-mode + validation/retention/classification/redaction units.
  `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.
- **TS domain layer — complete.** `src/lib/url/requestUrl.ts` (verbatim-encoding
  parse/serialize/reconcile), `src/lib/variables/resolve.ts` (env>global, depth 5, cycles),
  `src/lib/http/contentType.ts`, `src/lib/format/json.ts` (positioned errors),
  `src/lib/prepare/prepareRequest.ts` (+`types.ts`) — canonical pipeline with
  `PreparationOptions`, reserved no-op auth stage (`applyAuth`, activates M3a),
  sensitivity redaction. **63 Vitest tests green**; typecheck + lint clean.
- **UI foundation — done.** `src/stores/tabs.ts` (Tab model, `sendActiveTab` with
  stale-execution guard + retention release, `cancelActiveTab`, explicit dirty flag,
  `APP_DEFAULTS`), `src/composables/useHotkeys.ts` (mod = Cmd/Ctrl registry),
  `src/editor/{themes,extensions}.ts` (CM6 theme from `--rk-*` vars; Mod-Enter
  pass-through), `src/components/shared/CodeEditor.vue`,
  `src/components/request/{MethodSelect,UrlInput,SendButton,RequestTopBar,KeyValueEditor,BodyEditor,RequestSettings}.vue`.
- **IPC boundary** (`src/ipc/commands.ts`): all M1 wrappers exist. `src/ipc/errors.ts`
  has `AppError`/`friendlyMessage`.

### Remaining for M1 (in order)

1. **`src/components/request/RequestView.vue`** — column layout: `RequestTopBar` →
   sub-tab strip (Params | Headers | Body | Settings — Auth/Vars are M3a/M4, don't render)
   → active pane → `ResponsePanel` (bottom, ≥40% height). Shows
   `store.prepareErrors`/`prepareWarnings` above the editor. Wire `KeyValueEditor`
   granular events to the store:
   - Params pane: rows = `activeTab.draft.url.query` (QueryParam). On `edit(id, patch)`
     apply patch; if `patch.value` non-empty set `hasEquals = true`. On `add(patch)` push
     `{ id: crypto.randomUUID(), key: '', value: '', enabled: true, hasEquals: false, ...patch }`.
     On `remove(id)` filter out. Always `store.markDirty()`.
   - Headers pane: rows = `activeTab.draft.headers` (KeyValueRow); same pattern minus
     `hasEquals`.
2. **Response components** (`src/components/response/`):
   - `ResponsePanel.vue` — empty state / in-flight (spinner + cancel) / `ErrorDisplay` /
     success (ResponseMeta + Pretty|Raw|Headers toggle).
   - `ResponseMeta.vue` — status badge (2xx green, 3xx blue, 4xx orange, 5xx red),
     statusText, humanized duration (`245 ms`/`1.2 s`) and decoded size (`3.4 KB`),
     final URL (middle-truncate, title attr), badges for `bodyTruncated` and
     `downloadCapped`.
   - `ResponseBodyView.vue` — Pretty: JSON-ish contentType → `formatJson` in readonly
     CodeEditor (fallback raw); Raw: readonly text. `isBinary` → notice + Save button.
     Copy body via `writeText` from `@tauri-apps/plugin-clipboard-manager` (allowed
     tauri import here per plan's clipboard decision); Save via
     `chooseAndSaveResponse(response.executionId)` — `false` = user cancelled (silent),
     AppError = evicted (inline message).
   - `ResponseHeaders.vue` — table of `[name, value]` pairs, labeled "normalized headers".
   - `ErrorDisplay.vue` — `friendlyMessage(error)` headline, `detail` in collapsed
     `<details>`; kind `cancelled` renders muted, not error-styled.
3. **`src/components/layout/MainLayout.vue`** (M1: just RequestView full-window) and
   rewrite **`src/App.vue`**: render MainLayout, `useHotkeys().register('mod+enter', () => tabsStore.sendActiveTab())`.
4. **Component tests** (Vitest + happy-dom, `vi.mock('../../ipc/commands')`):
   send→cancel button flip; stale completion dropped + `releaseResponse` called for the
   stale id (resolve an older mocked promise after a newer send); KeyValueEditor
   trailing-blank-row `add` event; ErrorDisplay dns copy + muted cancelled.
5. **Verify:** `bun run test && bun run typecheck && bun run lint`, `cargo test` +
   clippy in `src-tauri/`, then `bun tauri dev` against `bun run fixtures`
   (http://localhost:4400): /json pretty-prints; /delay/10 cancel works; /gzip decoded;
   /dup-headers listed twice; /size/5000 truncation banner + save-to-file; /close shows
   connection error; `https://nope.invalid` shows DNS copy; bad JSON body shows lint
   position. Update `docs/smoke.md` M1 section to match PLAN.md's M1 exit list.
6. **Commit** (M1), push, confirm CI green on both OS runners.

## Context notes

- Three background subagents built the parallel layers but were killed by the account's
  **monthly spend limit** — don't re-spawn agents without checking the limit; finishing
  inline is fine (remaining work is small).
- `KeyValueEditor` deliberately emits events instead of mutating props
  (`edit`/`remove`/`add` — see its `<script setup>` types).
- Query/header text is stored **encoded verbatim** — never encode/decode in the UI.
- The URL bar is a projection: `serializeRequestUrl` out, `parseUrlBar` in (already wired
  in `UrlInput.vue`).
- `src/ipc/` and `src/types/` are the fixed contracts both sides compile against; change
  them only with matching Rust-side changes (`src-tauri/src/http/types.rs`).
- Rust modules are `pub` so `src-tauri/tests/http_integration.rs` can drive the executor
  directly.
- After M1: milestones M2a → M5 per PLAN.md. Delete this file once M1 lands.

## Decisions already made (do not re-litigate — see PLAN.md "Review history")

`~/.request-kit` storage (not app_data_dir); human-readable slug filenames with embedded
UUID identity; full MVP.md scope staged M0–M5; GraphQL body editor in scope (protocol
out); nested folders in M2, move/reorder M4; POSIX-shell cURL dialect; redirect limit
fixed at 10.
