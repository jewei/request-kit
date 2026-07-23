# request-kit — Implementation Plan

Revision 3.4 — consolidates five rounds of external review; every accepted change is folded into the body of this document. **`MVP.md` + `PLAN.md` are sufficient to start coding**; the `PLAN_FEEDBACK_*.md` files are historical artifacts and may be deleted. Scope is unchanged (everything in `MVP.md`), staged lean-first: **v0.1 = M0–M3**, remaining MVP.md scope lands in M4–M5.

## Context

request-kit is a greenfield, single-user, local-first desktop API client in the spirit of Postman/Bruno/Insomnia/Hoppscotch, built with Tauri. `MVP.md` (full feature scope) and this plan are together the complete implementation context. This plan covers **everything in MVP.md**; the MVP.md "Out of scope" list (scripting, tests, chaining, cookies, proxy, TLS config, WebSocket/gRPC protocol support, workspaces, accounts) stays out. The GraphQL *body editor* is in scope — it is a body mode, not protocol support.

**First action:** `git init` the repo and commit `MVP.md` and this plan.

## Review history

This plan absorbed five rounds of external review; everything accepted is integrated into the sections below — no other document is required. The final review verdict: no remaining architecture blockers; implementation (M0/M1) is the next useful source of feedback.

**Declined review suggestions (deliberate decisions — do not revisit without the user):**
- Storing under the platform `app_data_dir` — user chose `~/.request-kit`.
- UUID filenames — human-readable slugs kept; the embedded UUID is the identity.
- Removing the GraphQL body editor — MVP.md excludes GraphQL *protocol* support, not the body mode.
- Cutting scope to a lean v0.1 — user chose everything in MVP.md; it is staged (M0–M5), not cut.
- Deferring nested folders — storage is recursive directories either way; only move/reorder is deferred to M4.

## Decisions (locked)

- **Stack:** Tauri 2.x (Rust backend) + Vue 3 + TypeScript + Vite, **Bun** as package manager/script runner. No vue-router (single-window, no URL navigation).
- **HTTP execution in Rust** (reqwest) — avoids CORS, enables cancel, accurate timing, redirect/timeout control.
- **Persistence:** plain files on disk at `~/.request-kit` (`dirs::home_dir().join(".request-kit")`; user-chosen over the platform app-data dir — human-readable, Bruno-style. The live directory contains plaintext credentials and **should not be committed to git**; sanitized export is the supported version-control path for v0.1). Frontend never touches disk directly — all IO via Rust commands (single choke point for schemas, atomic writes, slug rules; no `tauri-plugin-fs`).
- **Editor:** CodeMirror 6, one reusable wrapper component.
- **Build targets: macOS and Windows.** Linux is explicitly ignored for now.
- **Identity:** the UUID inside each file is the stable identity everywhere in frontend state (tabs, history, references). Paths are storage detail; the workspace store maintains the id → path map. `type RequestReference = { id: string; path: string }`.
- **Single instance:** `tauri-plugin-single-instance`, **registered before all other plugins** (per Tauri docs). Second-launch callback: show → unminimize → focus the existing window. Required before any persistence work.
- **Toolchain:** `rust-toolchain.toml` pins a stable channel (≥ 1.85, reqwest 0.13's floor; pin the current stable at scaffold time) with `rustfmt` + `clippy` components.

## Versions (verified online 2026-07-23; exact pins live in lockfiles)

| Package | Range | Notes |
|---|---|---|
| tauri crate / CLI / @tauri-apps/api | 2.11.x | |
| Plugins: dialog 2.7.x (Rust-side only), window-state 2.4.x, single-instance 2.x, clipboard-manager 2.x | | window-state: pair with `visible:false` + show-on-ready. Webview capabilities are minimal — see Security boundaries. Opener is not used in v0.1; add it only when a feature needs it. |
| reqwest | 0.13.x | `default-features = false`, features: **`rustls, http2, stream, gzip, brotli, deflate`** (verified against docs.rs 0.13.4 — the feature is `rustls`, not `rustls-tls`; `http2` must be explicit once defaults are off). Add `multipart` in M4. No `json` (bodies arrive pre-serialized; responses parsed in frontend). No `charset` (we decode bytes ourselves; non-UTF-8 renders as binary with save-to-disk). Deliberately excludes `system-proxy` (default feature; proxy is out of scope) and cookies. |
| tokio 1.x, tokio-util 0.7 (CancellationToken), serde, serde_json, thiserror, uuid, dirs 5, chrono, tempfile | | |
| Vue | 3.5.x | do NOT adopt 3.6 RC (Vapor) |
| Vite | 8.1.x | Rolldown; `server.port: 1420`, `strictPort`, `envPrefix: ['VITE_','TAURI_ENV_']` |
| @vitejs/plugin-vue | 6.0.x | |
| Bun | 1.3.x | `beforeDevCommand: "bun run dev"`, `beforeBuildCommand: "bun run build"` |
| Pinia | 4.0.x | plain setup stores (trivially droppable to 3.x if issues) |
| CodeMirror | codemirror 6.x + @codemirror/{view,state,language,lang-json,lang-html,search,autocomplete,commands,lint}; cm6-graphql 0.2.x + graphql 16 (M4) | |
| Vitest 3.x, @vue/test-utils, typescript 5.x, vue-tsc | | |

## Platform targets + CI (foundation work, not an afterthought)

- `tauri.conf.json` bundle targets: `dmg`/`app` (macOS), `nsis` (Windows; NSIS over MSI). WebView2 `webviewInstallMode: downloadBootstrapper` (default) for Windows.
- **GitHub Actions matrix (macos-latest, windows-latest) lands in M0** and builds installers on every push. Code signing and publishing are deferred. CI verifies packaging; **GUI launch is verified by manually running the unsigned installers on both target systems** (a hosted job is not a reliable GUI-launch test).
- Paths: always `dirs::home_dir().join(".request-kit")` — never hardcoded separators. Atomic same-dir rename works on APFS and NTFS.
- Slugify for file names strips Windows-reserved characters (`<>:"/\|?*`), trailing dots/spaces, and reserved names (`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`); collisions get `-2`, `-3` suffixes. Case-only renames (`Users` → `users`) must be handled and tested — both target filesystems are typically case-insensitive.
- Shortcuts use a `mod` abstraction: Cmd on macOS, Ctrl on Windows.
- Manual smoke checklist (`docs/smoke.md`) runs on both OSes at every milestone exit.

## Project layout

```
request-kit/
├── package.json / bun.lock / vite.config.ts / tsconfig.json / index.html
├── rust-toolchain.toml
├── tools/fixture-server.ts       # Bun.serve dev fixture server (see Testing)
├── .github/workflows/build.yml   # macOS + Windows build matrix
├── src/                          # Vue frontend
│   ├── main.ts / App.vue
│   ├── ipc/commands.ts           # ONLY place calling invoke(); typed wrappers
│   ├── ipc/errors.ts             # AppError kind -> friendly message
│   ├── types/{request,response,workspace,history}.ts
│   ├── stores/{workspace,tabs,environments,settings,history,ui}.ts   # Pinia
│   ├── lib/                      # PURE TS, no Tauri imports — fully unit-testable
│   │   ├── prepare/prepareRequest.ts   # THE canonical request pipeline (see below)
│   │   ├── variables/resolve.ts  # scope merge + {{var}} substitution
│   │   ├── url/requestUrl.ts     # RequestUrl model: serialize / parse-back / reconcile
│   │   ├── curl/{tokenize,parse,generate}.ts   # generate in M3; tokenize/parse in M4
│   │   ├── codegen/wget.ts       # M5
│   │   ├── importexport/{postman,workspace}.ts # postman in M4
│   │   ├── http/{contentType,auth}.ts
│   │   └── format/json.ts        # pretty-print + positioned validation
│   ├── composables/{useHotkeys,useResolvedScope}.ts
│   ├── components/
│   │   ├── layout/{MainLayout,Sidebar,StatusBar}.vue
│   │   ├── sidebar/{CollectionTree,CollectionTreeNode,SidebarSearch,HistoryList}.vue
│   │   ├── tabs/{TabsBar,TabItem}.vue
│   │   ├── request/{RequestView,RequestTopBar,UrlInput,MethodSelect,SendButton,
│   │   │            RequestSubTabs,KeyValueEditor,BodyEditor,MultipartEditor,
│   │   │            GraphqlEditor,AuthEditor,RequestVarsEditor,RequestSettings}.vue
│   │   ├── response/{ResponsePanel,ResponseMeta,ResponseBodyView,ResponseHeaders,
│   │   │             HtmlPreview,ErrorDisplay}.vue
│   │   ├── environment/{EnvSelector,EnvironmentEditor}.vue
│   │   ├── palette/CommandPalette.vue          # M5
│   │   ├── settings/SettingsModal.vue
│   │   └── shared/{CodeEditor,Modal,ContextMenu,ConfirmDialog,InlineRename}.vue
│   ├── editor/{extensions,themes,varHighlight}.ts
│   └── styles/{base,themes}.css  # CSS custom props; [data-theme="dark"]
└── src-tauri/
    ├── Cargo.toml / tauri.conf.json / capabilities/default.json / icons/
    └── src/
        ├── main.rs / lib.rs / state.rs / error.rs
        ├── http/{types,clients,body,executor,error_map,retain}.rs
        ├── storage/{paths,atomic,workspace,requests,environments,settings,ui_state,history}.rs
        └── commands/{http,storage,files}.rs
```

The frontend must not import Tauri APIs outside `src/ipc/` and `src/main.ts` bootstrap.

## URL model (single source of truth)

The URL bar is an **editable projection**, not the source of truth. The canonical model:

```ts
interface RequestUrl { base: string; query: QueryParam[]; fragment: string }
interface QueryParam {
  id: string; key: string; value: string;
  enabled: boolean; description?: string;
  sensitive?: boolean;  // see Sensitivity metadata in the pipeline section
  hasEquals: boolean;   // distinguishes ?flag from ?flag=
}
```

- `RequestUrl` → displayed URL: `base` + serialized **enabled** rows + `fragment`. Variables in `base` may resolve to scheme, authority, and path, but **must not introduce `?` or `#`** — that is a validation error (prevents double-`?` serialization when e.g. `{{baseUrl}}` resolves to `https://example.com/api?tenant=123`). Query values belong in parameter rows.
- URL-bar edit → parse back **deterministically**: (1) parse enabled rows from the bar; (2) reuse existing enabled-row IDs by ordinal position; (3) update reused rows' key, value, and `hasEquals`; (4) create IDs for additional parsed rows; (5) remove unmatched enabled rows; (6) **preserve disabled rows untouched in their existing relative positions** (they simply don't appear in the bar). This keeps row identity and descriptions stable, including for duplicates like `?a=1&a=2`. Parse failures never destroy the model.
- **Encoding: rows store encoded substrings verbatim.** `QueryParam.key`/`value` hold the exact text as entered — `%XX` sequences, `+`, even malformed percent sequences are preserved, never decoded. URL-bar parse splits without decoding; row serialization writes stored values verbatim with no automatic percent-encoding. This guarantees lossless round-trips and keeps templates like `{{searchTerm}}` unambiguous. A decoded-value editing mode is a possible later feature.
- `prepareRequest` serializes `RequestUrl` **exactly once** into the transport URL.
- Parsing preserves duplicate keys, empty values, keys without `=` (`hasEquals`), fragments, percent-encoding, template variables, and values containing encoded delimiters. **Never use `new URL()`** — `{{baseUrl}}/x` is not a valid URL; string-split on `?`/`&`/`=`/`#`. All in `lib/url/requestUrl.ts`, exhaustively unit-tested.

This replaces the earlier conflicting "URL string is source of truth + separate params[]" design.

## Request preparation pipeline (canonical)

One pure function is the single source of truth for turning a draft into a sendable request. **Send, copy-as-cURL, preview, validation, and future codegen all consume its output** — this prevents drift between what is sent and what is generated.

```ts
prepareRequest(draft: RequestDraft, context: RequestContext): PreparedRequestResult
// context: { variableSources: VariableSources, appDefaults: { timeoutMs, followRedirects, maxBodyBytes } }
// (sources, not a pre-built scope — the pipeline owns scope building; nothing is "resolved" before substitution runs)
// result:  { ok: true, request: TransportRequest, warnings: Warning[] }
//        | { ok: false, errors: ValidationError[], warnings: Warning[] }
```

Pipeline order: clone draft → `buildScope(variableSources)` → resolve recursive variable values → substitute request fields (including auth fields) → **apply auth to structured headers and `RequestUrl.query`** (auth must precede URL serialization — API-key-in-query would otherwise require reparsing the serialized URL or bypassing the canonical model) → serialize selected body → infer Content-Type → serialize `RequestUrl` exactly once (enabled rows only) → validate the final transport request.

**Auth conflict rules:** an enabled manual `Authorization` header wins — configured auth is skipped and a warning is returned. Likewise, an existing enabled API-key header or query row wins over configured API-key auth, skipped with a warning.

**Sensitivity metadata:** `KeyValueRow.sensitive: boolean` records only the **explicit user choice**; effective sensitivity is computed, never stored: `row.sensitive || isSensitiveHeaderName(row.key) || row.origin === 'configuredAuth'`, where sensitive header names are `Authorization`, `Proxy-Authorization`, `Cookie` (case-insensitive). Renaming `Authorization` to `X-Custom` therefore drops automatic sensitivity while preserving any user-set flag. **Redaction operates on the normalized request structure, not only the `auth` object** — a manually entered credential row is redacted everywhere a configured one would be (cURL, exports, history, previews). Header-name and Content-Type matching is case-insensitive; query-key conflict matching stays case-sensitive.

Options are one coherent contract (replacing separate `maskSecrets`/`redactCredentials` flags, which could produce conflicting outcomes):

```ts
interface PreparationOptions {
  variableMode: 'resolve' | 'preserve';
  sensitiveValueMode: 'include' | 'redact';
  unresolvedMode: 'error' | 'warn';
}
```

| Consumer | variableMode | sensitiveValueMode | unresolvedMode |
|---|---|---|---|
| Send | resolve | include | error (explicit "send anyway" downgrades to warn) |
| Preview | resolve | redact | warn |
| Copy cURL (default) | preserve | redact | warn |
| Copy cURL with credentials (explicit warning) | resolve | include | warn |
| Future codegen | configurable | redact by default | warn |

Cycle/depth analysis runs even in `preserve` mode (broken variables surface as warnings); unresolved variables block only Send. `redact` replaces effectively-sensitive values (basic password, bearer token, API-key value, any effectively-sensitive row — see below) with `<redacted>`. Generated cURL targets **POSIX shell** (Bash, zsh, Git Bash, WSL); a PowerShell generator is a later addition. The redirect limit is fixed at 10 and is not a pipeline input (reqwest redirect policy belongs to the client; per-request limits would require extra clients for no v0.1 benefit).

## Rust HTTP backend

**Shared clients** (`http/clients.rs`): two reusable `reqwest::Client`s in `AppState`, built once at startup — `follow_redirects` (policy `limited(10)`) and `no_redirects` (policy `none`). Clients are reused per reqwest's own recommendation (each owns a connection pool). Per-request timeout uses `RequestBuilder::timeout`, not client rebuilds. **Both clients call `ClientBuilder::no_proxy()`** — cargo features are additive across the dependency graph, so omitting `system-proxy` alone does not guarantee proxy-free behavior. Never set `danger_accept_invalid_certs`; no cookie store.

**Command surface** (all return `Result<T, AppError>`; `AppError = { kind, message, detail? }`; serde camelCase):

- `send_request(payload)` → `HttpResponseData`. Payload: `{ executionId (uuid), tabId, method, url (pre-resolved), headers: [{name,value}], body, timeoutMs|null, followRedirects, maxBodyBytes }`. Body tagged union: `none | text{content} | multipart{parts} | file{path}` (multipart/file arrive in M4) — JSON/raw/urlencoded/GraphQL all arrive pre-serialized as `text` with Content-Type already set by `prepareRequest`.
- `cancel_request(executionId)`, `release_response(executionId)`.
- **Dialog-owning file commands** (Rust opens the native dialog and immediately performs the IO — **no arbitrary paths cross IPC**): `choose_and_save_response(executionId)`. **Import is a two-step backend-owned flow:** `inspect_workspace_import()` opens the dialog, validates the file (20 MB cap), fully stages it, and returns `ImportPreview { importId, collections, requests, environments, warnings, containsRecognizedSensitiveValues, rawBodiesIncluded }` (honest naming — request-kit cannot detect credentials inside arbitrary raw bodies); after user confirmation, `apply_workspace_import(importId)` performs the replacement and returns a fresh workspace bootstrap (**rejected while any other storage mutation is active**); `cancel_workspace_import(importId)` discards staging. **Export:** `choose_and_export_workspace({ includeSensitiveValues })` builds the export **from backend storage** (saved state only) — never from a frontend payload, which could be stale, partially edited, or inconsistently redacted. Strict symlink/`..` rejection applies inside `~/.request-kit`; dialog-selected external paths are used as chosen.
- Storage: `load_workspace` (one bootstrap call: tree + envs + globals + settings + uiState), `create_collection/folder/request`, `read_request(id)`, `write_request`, `rename_node(id, newName)`, `delete_node(id)`, `duplicate_request(id)`, `move_node(id, newParentId)` (M4), `write_collection_meta`, `list_environments`, `write_environment`, `delete_environment`, `read_globals/write_globals`, `read_settings/write_settings`, `read_ui_state/write_ui_state`, `read_history(limit)/append_history/clear_history`.

**IPC input is untrusted.** `send_request` validates before touching reqwest and returns typed validation errors (never raw builder errors): executionId/tabId format; method against a supported-method allowlist; URL scheme must be `http`/`https`; header names/values well-formed; timeout within 1 ms–10 min; `maxBodyBytes` within 1–100 MB; text body ≤ 10 MB; ≤ 200 header rows and ≤ 256 KB aggregate header bytes. This also stops a damaged settings file from requesting an absurd response allocation.

**Send/cancel:** `AppState.abort_map: Mutex<HashMap<ExecutionId, CancellationToken>>`; executor races the request future against the token in `tokio::select!`; RAII guard removes the entry on drop. **The frontend ignores any completion whose `executionId` is not the tab's latest execution** — stale/cancelled sends can never update the UI.

**Timing/size:** duration = application-observed elapsed time from just before execute to last body chunk (no wire-accuracy claim). Size = **decoded body bytes** received.

**Response headers are normalized, not wire-original:** with automatic gzip/brotli/deflate decoding, reqwest removes `Content-Encoding` and `Content-Length` from the header map. The UI labels these as "normalized response headers" / "decoded body size"; retaining wire-original headers would require manual decoding and is explicitly not a v0.1 goal.

**Text vs binary classification (deterministic):** textual `Content-Type` (`text/*`, `application/json`, `application/*+json`, `application/xml`, `application/*+xml`, `application/javascript`, `application/graphql`) → attempt UTF-8 decode; invalid UTF-8 ⇒ binary. Clearly binary types (`image/*`, `audio/*`, `video/*`, `application/octet-stream`, archives, PDFs, fonts) ⇒ binary even when bytes happen to be valid UTF-8. Missing/unknown type ⇒ text iff valid UTF-8 with no NUL bytes. Non-UTF-8 "textual" responses show: *"This response is not valid UTF-8. Save it to inspect the original bytes."* (charset conversion is intentionally excluded).

**Response limits & retention** (`http/retain.rs`): download hard-capped at `maxBodyBytes` (default 10 MB, settable) → reading stops, response marked `downloadCapped`. IPC returns ≤ 2 MB UTF-8 text (`bodyTruncated`; `isBinary` → `body: null`). Retention is **keyed by execution ID** (not tab — a slow earlier request finishing late must never overwrite a newer response):

```rust
// HashMap<ExecutionId, Bytes> + FIFO order queue, under one lock:
// max 50 MB total, max 10 responses — oldest evicted first;
// insertion + eviction are a single synchronized operation, so
// concurrent completions can never exceed either budget.
// No tab_id — all operations are execution-keyed and the frontend
// owns tab cleanup, keeping the backend independent of UI identity.
```

**Retention lifecycle** (`release_response` is **idempotent** — stale-completion cleanup, FIFO eviction, and tab cleanup may release the same execution more than once, silently): a stale completion is ignored by the UI **and immediately released** (`release_response(staleExecutionId)`); an accepted new completion releases the previously displayed execution before retaining the new one; tab close cancels any in-flight execution and releases the displayed one; failed or cancelled sends retain no bytes. `choose_and_save_response(executionId)` writes the retained bytes via a Rust-owned save dialog; evicted → typed error prompting re-send. Disk spooling for larger downloads is deferred (M4).

**`HttpResponseData`:** executionId, status, statusText (**the canonical label for the status code, not necessarily the server's original reason phrase**), httpVersion, header pairs (**duplicate values preserved; global ordering unspecified** — reqwest's `HeaderMap` iteration order is explicitly arbitrary), durationMs, bodyBytes, contentType, finalUrl, body, bodyTruncated, isBinary, downloadCapped.

**Error model** (`error_map.rs`):

```
ErrorKind = invalidUrl | cancelled | timeout | tls | dns | connection | io | unknown
```

Non-2xx HTTP responses are normal responses, never errors. `classify(&dyn Error)` walks the `source()` chain: reqwest `is_timeout` → `timeout`; `is_builder`/URL parse → `invalidUrl`; `rustls::Error` in chain → `tls`; `io::ErrorKind::{ConnectionRefused, ConnectionReset, BrokenPipe}` → `connection`; DNS is **best-effort** (resolver error detection varies by OS — fall back to `connection` when ambiguous, never mis-claim). **`AppError.detail` is redacted before crossing IPC: URL query values and userinfo credentials are stripped from any URLs appearing in the error chain** (a resolved URL containing an API key must never surface in error text). The unredacted chain exists only in memory. Finer categories are added only when fixture tests demonstrate consistent macOS + Windows behavior. Frontend maps kinds to friendly copy (MVP requirement: clear DNS/TLS/timeout/connection messages).

**Bodies (M4):** multipart via `reqwest::multipart::Form` (enable the `multipart` feature then), file parts streamed (`Part::stream` over `tokio::fs::File`) — never fully in memory; binary body via `ReaderStream`; no manual Content-Type for multipart (boundary).

## Storage design (`~/.request-kit`)

All files JSON with top-level `"version": 1`. Loader: newer version → clear error; migration infrastructure is added only when the first schema change actually occurs.

**Atomic writes** (`storage/atomic.rs`):

```rust
let mut temp = tempfile::NamedTempFile::new_in(parent)?;   // same dir as target
serde_json::to_writer_pretty(temp.as_file_mut(), value)?;
temp.as_file_mut().flush()?;
temp.as_file().sync_all()?;    // flush() alone is NOT durable — must fsync
temp.persist(target)?;
// then best-effort fsync of the parent directory on Unix; skip on Windows
```

All replacement goes through this single helper — `NamedTempFile::persist` over an existing file is verified on both platforms during M0, and a Windows-specific replacement strategy can be swapped in without touching callers. Original file preserved if replacement fails. Symlinks inside app-managed storage are rejected. **File permissions:** on Unix, storage directories are created mode `0700` and user-data files `0600`; Windows relies on inherited per-user ACLs. **A corrupt individual file is quarantined (renamed `.corrupt-<ts>`) and reported — it never prevents app startup.** **Duplicate embedded UUIDs** (possible when users hand-copy files): the scan never silently picks one — discovered paths are **normalized to `/`-separated relative form, sorted lexicographically, and the first is authoritative** — the same file wins on Windows and macOS; the rest are quarantined and reported with their locations and a recovery action (deterministic across machines — filesystem iteration order is not).

```
~/.request-kit/
├── settings.json      # theme system|light|dark, fontSize, timeoutMs, followRedirects,
│                      # maxBodyBytes, editorLargeFileKb
├── ui-state.json      # openTabs [{id,pinned}], activeTabId, activeEnvId,
│                      # sidebarWidth, sidebarVisible, expandedFolderIds
├── globals.json       # variables
├── environments/<slug>-<id8>.json    # { id, name, variables }
├── collections/<collection-slug>/
│   ├── collection.json               # { id, name, auth, variables }
│   ├── <request-slug>.json           # RequestFile
│   └── <folder-slug>/ (folder.json + requests, recursive)
└── history/history.jsonl             # append-only; compacted at startup past 500 entries
```

- **Identity:** UUID inside the file is authoritative; the workspace scan builds the id → path map. Rename = update `name` in JSON + `fs::rename` to new slug (references unaffected — they hold ids); case-only renames handled via two-step rename on case-insensitive filesystems. Duplicate = new uuid + " copy" — **duplicate applies to requests only in v0.1; recursive folder/collection duplication is deferred.** Ordering is **alphabetical (case-insensitive) in v0.1**; manual `order` arrays + move/drag arrive in M4.
- `Variable = { key, value, enabled, secret }`.
- **Plaintext disclosure (stated in settings UI): variable values AND request auth values — basic-auth passwords, bearer tokens, API keys, and anything typed into bodies — are stored unencrypted under `~/.request-kit`.** Masking is UI/serialization behavior only. Keychain integration is a documented future item. The directory should not be committed to git; sanitized export is the supported version-control path.
- **RequestFile schema:** `{ version, id, name, method, url: RequestUrl (see URL model), headers[] (key/value/enabled/description/sensitive rows), body { mode: none|raw|json|formUrlencoded|multipart|binary|graphql, per-mode fields }, auth { type: inherit|none|basic|bearer|apikey{in: header|query} }, variables[], settings { timeoutMs|null, followRedirects|null } }` (null = inherit app settings; `inherit` auth resolves to `none` until M4 delivers collection/folder inheritance).
- **History entry:** `{ version, id, executedAt, method, templateUrl, status, durationMs, bodyBytes, requestId|null, errorKind|null }`. **Redaction rules (one rule for all requests): the history URL is the draft `RequestUrl` serialized before variable substitution, with every literal enabled query value redacted and `{{variable}}` placeholders preserved. Request headers and resolved secrets are never written. No guessing which parameter names are confidential, and no scratch-request special case — every request has a draft `RequestUrl`.** JSONL tolerates a torn last line.
- **Own export format:** `{ version, kind: "request-kit-workspace", exportedAt, collections (inlined tree), environments, globals }`. **Default export blanks all credentials: secret variable values, `basic.password`, `bearer.token`, `apikey.value`, and values of rows marked `sensitive`** (literal auth values are credentials even when not backed by a secret variable). The `includeSensitiveValues` opt-in confirmation restores all of them. Raw request bodies may contain credentials request-kit cannot identify reliably — the export confirmation states that bodies are included unchanged.
- **Import semantics (v0.1): full replacement, crash-recoverable.** `inspect_workspace_import` validates and **fully stages** into `~/.request-kit.import-staging-<id>`. The transaction marker `~/.request-kit-import-transaction.json` is explicit: `ImportTransaction { version, importId, phase, stagingPath, backupPath, startedAt }`, `phase ∈ Staged | LiveMovedToBackup | StagingMovedToLive | Verified`; the marker, staging dir, and backups receive the same restrictive permissions as the live library. `apply_workspace_import`: write + fsync marker (`Staged`) → rename current library to `~/.request-kit.backup-<timestamp>` (`LiveMovedToBackup`) → rename staging to `~/.request-kit` (`StagingMovedToLive`) → reload and verify (`Verified`) → clear marker. **Startup recovery is deterministic per phase** (directory replacement is not one indivisible operation on either OS): `Staged` → remove abandoned staging (or resume after confirmation); `LiveMovedToBackup` → install valid staging, else restore backup; `StagingMovedToLive` → verify live library, else restore backup; `Verified` → clear marker + apply backup retention. **Frontend preconditions before `apply`:** prompt to save/discard dirty tabs → cancel all in-flight requests → release all retained responses → flush or cancel pending debounced UI-state writes → apply → replace all stores from the returned bootstrap → open a clean scratch tab if nothing restores. **Global maintenance state:** `AppMode = Normal | ImportStaging | ImportApplying | Recovery` in `AppState`; during `ImportApplying`/`Recovery` the backend rejects new sends, storage mutations, history appends, and environment/settings/UI-state writes with a typed `maintenanceInProgress` error (import cancellation permitted when safe) — preconditions alone can't stop a send racing the replacement. The frontend disables sending and storage controls before calling `apply`. Staging directories with no active transaction marker and older than 24 h are cleaned up at startup. **Backup retention: keep only the latest 3** (backups contain the same plaintext secrets); backups are deletable from Settings. Merge import is a later feature.

## Frontend architecture

**Stores:** `workspace` (tree of nodes keyed by id, id→path map, CRUD via IPC, in-memory search over name+url+method); `tabs` (Tab = `{ tabId, requestId|null (null = scratch), draft, pinned, dirty, response, responseError, inFlightExecutionId }` — **dirty is an explicit flag** set on first edit, cleared on save; no continuous deep-diffing. Completions with a stale executionId are dropped and released. Saved tabs persist `{id,pinned}` to ui-state debounced and restore on boot; scratch tabs are not persisted in v0.1 and warn before close/quit); `environments` (envs, globals, activeEnvId); `settings`; `history`; `ui`. **Deleting a saved request or containing folder:** confirmation prompt; affected open tabs — dirty or clean — convert to unsaved scratch tabs (one consistent rule; edits are never silently lost); their retained responses are released and their ids removed from persisted ui-state.

**Component tree:** `MainLayout` (grid: sidebar | main). Sidebar: Collections/History tabs, recursive `CollectionTreeNode` with context menus (rename/delete/new; duplicate on requests), search. Main: `TabsBar` → `RequestView` (top bar: method + URL + send/cancel; sub-tabs: Params/Headers/Body/Auth/Vars/Settings — Params edits `RequestUrl.query` rows, URL bar is the projection; bottom: `ResponsePanel` with Pretty | Raw | Headers toggle — Preview added with sandboxed HTML in M4). `EnvSelector` in top bar; env editor, settings, command palette (M5) are teleported overlays.

**Clipboard:** `tauri-plugin-clipboard-manager` with only `clipboard-manager:allow-write-text` granted (webview `navigator.clipboard` permission behavior differs between WKWebView and WebView2 — don't rely on it). Used for copy-body and copy-as-cURL.

**CodeMirror wrapper** (`shared/CodeEditor.vue`): props `modelValue, language (text|json|html|graphql), readonly, lineNumbers, enableVars, largeTextMode`; Compartments for language/readonly/theme/fontSize; high-priority keymap pass-through so mod+Enter/mod+S reach global hotkeys. Large-response fallback: > `editorLargeFileKb` (1 MB default) → plain text mode; truncated bodies show a banner: "Response truncated for display — Save to file for full body."

**Hotkeys** (`useHotkeys.ts`): single window keydown listener + declarative registry (`mod` = Cmd/Ctrl): mod+Enter send, mod+S save, mod+N new, mod+K palette (M5), mod+P quick-open (M5 — the palette in open-request mode; no separate v0.1 overlay), mod+W close tab, mod+Shift+E env switch, mod+, settings, mod+B sidebar. Registry feeds shortcut hints in UI.

**Theme:** CSS custom props under `:root` / `[data-theme="dark"]`; CM theme built from the same `var(--rk-*)`; "system" via matchMedia listener.

## Variables

Precedence is staged. **v0.1:**

```
active environment > global
```

**M4** extends to the full MVP chain: `request > folder (innermost first) > collection > environment > global`.

Precise rules (all versions):
- Keys are case-sensitive. Disabled variables do not resolve.
- Recursive values resolve to a **maximum depth of 5**; cycles produce a validation error.
- Variables are substituted in: URL, query keys and values, header names and values, body text, and auth fields. Variables in `RequestUrl.base` must not introduce `?` or `#` (validation error; see URL model).
- Unresolved variables **block sending by default** with a clear list of missing names; the user can explicitly "send anyway".
- Secret values are masked in editors, previews, tooltips, history, logs, and exports (masking is UI/serialization behavior; disk storage is plaintext, see Storage).

Used three ways: CM inline decorations (resolved = pill + tooltip, masked if secret; unresolved = red), `{{` autocomplete with source badges, and `prepareRequest` resolution — **Rust only ever sees resolved strings**.

## Security boundaries

- No remote/response content is ever injected into the app DOM. HTML preview (M4) uses `<iframe sandbox srcdoc>` with an empty sandbox attribute **plus a restrictive CSP `<meta>` injected into the srcdoc** (`default-src 'none'; img-src data: blob:; style-src 'unsafe-inline'; font-src data:`) so the document can't fetch external resources.
- File operations on user-chosen paths go through **dialog-owning Rust commands** — arbitrary paths never cross IPC. Strict symlink/`..` rejection inside `~/.request-kit`.
- Storage permissions: `0700` dirs / `0600` files on Unix; per-user ACLs on Windows. `~/.request-kit` is not for git; sanitized export is the version-control path.
- Tauri capabilities (webview-exposed): only the core permissions the frontend actually calls, plus `clipboard-manager:allow-write-text`, plus window-state permissions only if invoked from JS. **No `dialog:*` permissions** — dialogs open exclusively inside Rust-owned commands, and registering a plugin for Rust-side use does not require exposing it to the webview. **No opener plugin in v0.1.** Add permissions only when a frontend call actually needs them.
- Single instance enforced (registered first; see Decisions). Request/response content never written to logs.
- Secrets: template-first history, credential-blanking exports, credential-redacting cURL, redacted `AppError.detail` (see Storage / Error model / prepareRequest).

## Milestones

v0.1 = M0–M3. Each milestone exits with `cargo test` + `bun run test` green and the smoke checklist run on macOS and Windows. M2/M3 are split into internal sub-milestones (a/b) to keep PRs and verification tractable; the public v0.1 boundary is unchanged.

**M0 — Foundation.** `git init` + initial commit; scaffold (Tauri 2 + Vue 3 + Vite 8 + Bun); `rust-toolchain.toml`; ESLint/Prettier/rustfmt/clippy; Vitest + cargo test wiring; GitHub Actions matrix building both installers; single-instance plugin registered first (show/unminimize/focus callback); storage root resolution; core IPC types (TS + Rust, camelCase serde); `tools/fixture-server.ts`; window `visible:false` + show-on-ready.
*Exit: installers build in CI **and** unsigned installers are launched manually on both target systems.*

**M1 — Send one request.** Method/URL editors on the `RequestUrl` model (bar-as-projection + deterministic reconciliation); header rows; body modes none/raw/JSON (**Content-Type in M1: JSON → `application/json` when absent; raw text → none set automatically**); `prepareRequest` pipeline (the auth stage exists as a **reserved no-op** until M3a); shared reqwest clients with `no_proxy()`; send/cancel/timeout/redirect toggle; stale-execution guard + retention lifecycle; response panel (status, normalized headers, duration, decoded size, final URL, raw + formatted JSON, copy body via clipboard plugin); download/IPC limits + truncation warnings; executionId-keyed retention + save-response dialog command; error model with redacted detail; Rust-side IPC payload validation + limits; text/binary classification; CodeEditor wrapper (text/json).
*Exit: fixture suite covers JSON echo, delay+cancel, redirect chain, gzip, dup headers, oversize body, malformed URL, abrupt close — all rendering correct status/error UI; a slow response finishing after a newer send neither clobbers the display nor lingers in retention.*

**M2a — Storage and saved requests.** `storage/` module + commands; `~/.request-kit` layout with `0700`/`0600` permissions; atomic writes (fsync); corrupt-file + duplicate-UUID quarantine (lexicographic determinism); collections + nested folders (create/rename/delete; **duplicate = requests only**; alphabetical order; case-only rename handling); UUID identity + id→path map; explicit dirty state + close warnings; retention wired to tab close (`release_response`).
*Exit: create → save → quit → relaunch → rename → delete works on both OSes without data loss; corrupt and duplicate-id files quarantine cleanly and deterministically.*

**M2b — History, search, settings, themes, window state.** History (redacted, JSONL, startup compaction) + sidebar list; search; settings + SettingsModal (incl. backup management); themes; window-state restore.
*Exit: history shows template URLs only; theme/window state survive relaunch.*

**M3a — Tabs, variables, authentication.** Multiple tabs (pin/close/dirty dots/restore saved tabs); global + environment variables with validation, masking, autocomplete, inline preview; block-on-unresolved with override; auth basic/bearer/apikey **implemented and activated in the pipeline's reserved auth stage**, with conflict warnings and sensitivity auto-marking.
*Exit: a saved request using `{{baseUrl}}` + bearer auth reopens and sends; unresolved variable blocks with a clear message; a manual Authorization header beats configured auth with a visible warning and is redacted in default cURL output.*

**M3b — Interchange, hotkeys, editor polish (completes v0.1).** Copy-as-cURL via `prepareRequest` (credential-redacting default + explicit "with credentials" variant, POSIX dialect); own-format export (credential-blanking default, `includeSensitiveValues` opt-in) + two-step replacement import with import preconditions, phased transaction marker, startup recovery, and backup retention; **v0.1 hotkey set** (mod+K/mod+P arrive with the palette in M5); JSON lint + format command; CM search/replace; large-response plain-text fallback.
*Exit: **Copy cURL with credentials** replays identically in a POSIX terminal; export→wipe→import restores the library via the two-step flow; simulated interruption mid-replacement recovers on next startup; backups pruned to 3.*

**M4 — Extended MVP scope.** Each item lands individually with its own schema/tests/checklist: multipart + file upload + binary bodies (add reqwest `multipart` feature); GraphQL body editor (cm6-graphql); **automatic Content-Type for the new modes (form-urlencoded, GraphQL, multipart boundary, binary)**; folder/collection/request variable scopes (full precedence) + inherited auth; cURL import (hand-rolled tokenizer/parser — npm parsers are unmaintained and wrong on quoting; ~300 LOC, warning list for unknown flags); Postman v2.1 subset import with per-item warnings; drag-and-drop import (`onDragDropEvent`, sniff `kind` else try Postman); move/reorder nodes (`order` arrays) + folder/collection duplication; sandboxed HTML preview (CSP srcdoc); large-response disk spooling.
*Exit per feature: its fixture/unit suite + smoke entry passes on both OSes.*

**M5 — Power features + release polish.** Codegen panel (cURL + wget, template-based, resolved-vs-template toggle with credential redaction); CommandPalette (open request fuzzy [also bound to mod+P], switch env, send, format body, copy as cURL, toggle sidebar, settings); empty states, toasts; `docs/smoke.md` complete; icons/bundle metadata; release build verification on both OSes.
*Exit: every palette action works; both installers launch with restored state.*

## Testing

**Fixture servers.** Rust integration tests start an **in-process fixture server bound to port 0** (raw `TcpListener` + minimal handcrafted HTTP — well suited to abrupt-close and malformed-response cases); `cargo test` needs no external orchestration. The Bun server (`tools/fixture-server.ts`, Bun.serve, dev-only) serves interactive development and the manual checklist: JSON echo, `/delay/:secs`, `/redirect/:n`, duplicate headers, gzip, binary, `/size/:kb`, abrupt close, multipart echo (M4). External services (httpbin) appear only in the optional manual checklist.

- **Rust (`cargo test`):** cancellation cleanup (abort map empty after cancel/complete/error); stale-execution retention isolation (older completion never overwrites newer entry; stale entries released); FIFO retention budget under concurrent completions; timeout and redirect behavior against the in-process fixture; body cap; IPC payload validation limits (method allowlist, scheme, header count/size, timeout/body ranges → typed errors); text/binary classification matrix; maintenance-mode rejection (sends/writes during `ImportApplying`); error classification against constructed chains; `AppError.detail` redaction (URL with query secrets); atomic replacement (no temp residue; original survives simulated failure; sync invoked via the storage abstraction — tests verify the call and simulated-crash recovery, not physical durability, which a unit test cannot prove); corrupt-file and duplicate-UUID quarantine (deterministic ordering); slugify + Windows reserved names + case-only rename; UUID identity through rename; history redaction (no auth headers, template URLs, literal query values redacted with `{{var}}` preserved); symlink rejection; future-version rejection; idempotent `release_response`; import replacement (staging failure → rollback restores backup; **simulated interruption at each `ImportPhase` → startup recovery yields a consistent library**; apply rejected during concurrent mutation); backup pruning.
- **TS (Vitest, `bun run test`):** `prepareRequest` stage ordering (auth before URL serialization) and option matrix incl. credential + sensitive-row redaction (auto-marking of Authorization/Proxy-Authorization/Cookie, case-insensitive header matching); `requestUrl` serialize/parse-back/reconcile (duplicates, empties, `hasEquals`, fragments, verbatim encoded-substring round-trips — `%20` vs `+` vs malformed sequences, templates, disabled-row preservation, deterministic ID reuse, parse-failure safety, base-with-`?` validation error); variable precedence, recursion depth, cycles, disabled vars, case sensitivity; auth conflicts (manual header wins + warning, per pipeline rules); Content-Type inference; cURL generation quoting incl. `<redacted>` placeholders (+ parse round-trip in M4); Postman fixtures (M4); workspace export round-trip incl. credential blanking; dirty-state transitions.
- **Component tests** (Vitest + mocked `ipc/commands.ts`): send/cancel button states, stale-completion dropping, save + dirty flag, closing dirty tabs, URL-bar↔rows sync, environment switching.
- **E2E: none automated** — tauri-driver/WebDriver doesn't cover macOS. Manual smoke checklist (`docs/smoke.md`) per milestone on both OSes; unsigned installers manually exercised at every release candidate, starting at M0.

## Risks & gotchas

- **IPC payload size:** mitigated by 2 MB display cap + executionId-keyed retention (50 MB / 10 responses FIFO) + capped downloads.
- **window-state:** restore-at-init + `visible:false` + explicit show, exclude VISIBLE flag; verify off-screen clamping in M2b smoke.
- **reqwest 0.13:** features verified against docs.rs (`rustls`, explicit `http2`); re-check when bumping; don't copy 0.12-era snippets (`rustls-tls` etc. no longer apply). `no_proxy()` is the behavioral guarantee — the feature omission alone is not.
- **DNS classification is best-effort** — resolver errors differ across OSes; fall back to `connection`, never fabricate specificity.
- **Pinia 4 is new:** plain setup stores keep a drop to 3.x one-line.
- **CM6 `{{var}}` decorations:** MatchDecorator-based ViewPlugin, language-agnostic.
- **Windows:** WebView2 bootstrapper for fresh machines; NSIS target; reserved-filename slugs; case-insensitive rename handling; Ctrl for `mod`; NTFS atomic rename same-volume only (all writes are same-dir, fine).
- **Open decision (non-blocking, post-v0.1):** whether collections eventually live in user-selected git project folders ("Open project folder") in addition to the app-managed `~/.request-kit` library. v0.1 assumes app-managed only.

## Critical files

- `src/lib/prepare/prepareRequest.ts` — the canonical pipeline; send/cURL/preview/validation all consume it
- `src/lib/url/requestUrl.ts` — the URL single-source-of-truth model
- `src-tauri/src/http/executor.rs` — send/cancel/timing/cap core
- `src-tauri/src/http/retain.rs` — executionId-keyed retention + FIFO budget + lifecycle
- `src-tauri/src/storage/requests.rs` — on-disk CRUD + slug/atomic-write/quarantine contract
- `src/ipc/commands.ts` — the single typed IPC boundary
- `src/stores/tabs.ts` — draft/dirty/send/response state machine
- `src/lib/variables/resolve.ts` — precedence + substitution rules
