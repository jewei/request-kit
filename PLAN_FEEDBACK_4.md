## Verdict

Revision 3.1 is now **implementation-ready at the architectural level**. The major correctness and security concerns from the previous rounds have been resolved, including auth-before-URL serialization, proxy disabling, crash-recoverable imports, response retention cleanup, credential-aware exports, deterministic UUID handling, and filesystem permissions. 

I would begin M0 after making a few smaller specification patches.

## Remaining important fixes

### 1. Manual credentials can still bypass redaction

The plan redacts configured Basic, Bearer, and API-key authentication. However, the auth conflict rule says a manually entered `Authorization` header or API-key parameter wins over configured auth. 

That manually entered value is stored as an ordinary header or query row, so it may still appear in:

* Default cURL output
* Sanitized exports
* History URLs
* Search indexes or previews

Add sensitivity metadata to key/value rows:

```ts
interface KeyValueRow {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
  sensitive?: boolean;
}
```

Automatically mark these as sensitive:

```text
Authorization
Proxy-Authorization
Cookie
configured API-key header/query row
```

Users should also be able to mark custom rows as sensitive. Redaction should operate on the normalized request structure, not only the `auth` object.

For history, the safest lean rule is:

```text
Store query keys, but redact every literal query value.
Preserve {{variable}} placeholders.
```

This avoids trying to guess whether `token`, `key`, `signature`, or an arbitrary custom parameter is confidential.

### 2. Define import preconditions for open tabs

The replacement import flow is now robust on disk, but the frontend may still have:

* Dirty tabs
* Active requests
* Retained response bodies
* Saved tabs referring to IDs that will disappear
* Debounced UI-state writes waiting to run

The plan should specify this sequence before `apply_workspace_import`: 

```text
1. Prompt the user to save or discard dirty tabs.
2. Cancel all in-flight requests.
3. Release all retained responses.
4. Cancel pending storage/UI-state debounce operations.
5. Apply the import transaction.
6. Replace frontend stores with the returned bootstrap.
7. Open a clean scratch tab if no restored tab exists.
```

The backend should reject `apply_workspace_import` if another storage mutation is active.

### 3. Make the import transaction marker explicit

The transaction process is sound, but “startup recovers to either staging or backup” is still underspecified. 

Define the marker:

```rust
struct ImportTransaction {
    version: u32,
    import_id: Uuid,
    phase: ImportPhase,
    staging_path: PathBuf,
    backup_path: PathBuf,
    started_at: DateTime<Utc>,
}

enum ImportPhase {
    Staged,
    LiveMovedToBackup,
    StagingMovedToLive,
    Verified,
}
```

Recovery can then be deterministic:

```text
Staged
→ remove abandoned staging or resume after confirmation

LiveMovedToBackup
→ install valid staging; otherwise restore backup

StagingMovedToLive
→ verify live library; otherwise restore backup

Verified
→ remove marker and apply backup-retention policy
```

The staging directory, backup directories, and transaction marker should receive the same restrictive permissions as the live library.

### 4. Reduce frontend Tauri capabilities

All file dialogs are now opened inside Rust-owned commands. Therefore, the webview should not need `dialog:default`. The plan also grants `opener:default`, but no v0.1 feature appears to use the opener plugin. 

For v0.1, the capability set can likely be reduced to:

```text
core permissions actually required by the frontend
clipboard-manager:allow-write-text
window-state permissions only if invoked from JavaScript
```

Registering a plugin for Rust-side use does not mean its commands must be exposed to the webview. Add dialog or opener permissions later only when a frontend call actually requires them.

### 5. Clarify variable-scope ownership

The function signature says the context already contains:

```ts
variables: ResolvedScope
```

but the pipeline begins by building the variable scope. 

Choose one contract.

Preferred:

```ts
interface RequestContext {
  variableSources: VariableSources;
  appDefaults: AppDefaults;
}
```

Then:

```text
prepareRequest
→ buildScope(variableSources)
→ resolve recursive variable values
→ substitute request fields
```

Alternatively, pass an already-built `VariableScope` and remove “build variable scope” from the pipeline. Avoid calling it `ResolvedScope` because field substitution has not occurred yet.

### 6. Align milestone wording

M1 says the pipeline includes authentication ordering, while authentication itself is introduced in M3a. 

Use:

```text
M1:
prepareRequest includes a reserved auth stage that is a no-op.

M3a:
implement and activate Basic, Bearer, and API-key transformations.
```

Similarly, M3b says “full hotkey set,” but `mod+K` and `mod+P` are explicitly deferred to M5. Change that phrase to:

```text
v0.1 hotkey set
```

### 7. Rename credential-related API fields

`includeSecrets` now controls secret variables **and** authentication credentials. `ImportPreview.containsCredentials` cannot reliably detect credentials inside arbitrary raw request bodies. 

Clearer names would be:

```ts
choose_and_export_workspace({
  includeSensitiveValues: boolean
})

interface ImportPreview {
  containsRecognizedSensitiveValues: boolean;
  rawBodiesIncluded: boolean;
}
```

This avoids implying that request-kit can identify every credential embedded in free-form text.

## Minor implementation notes

`release_response(executionId)` should be idempotent. Stale completions, FIFO eviction, and tab cleanup may attempt to release the same execution more than once; this should silently succeed.

The retained-response structure probably does not need `tab_id`, because all operations are execution-ID based and the frontend controls tab cleanup. Removing it keeps the backend independent of UI identity.

Content-Type detection and auth-header matching must be case-insensitive, while query-key conflicts should remain case-sensitive.

The deterministic duplicate-UUID scan should sort normalized relative paths using `/` separators, rather than native path display strings, so Windows and macOS choose the same authoritative file.

## Final assessment

There are no remaining architectural blockers. The seven patches above mainly tighten contracts and prevent edge-case leakage or state desynchronization. After those edits, further plan-review rounds will likely provide diminishing returns; implementation and tests should become the primary source of feedback.
