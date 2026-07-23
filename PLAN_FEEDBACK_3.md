## Overall verdict

Revision 3 is much stronger and is close to implementation-ready. It correctly incorporates the previous reviews: the URL model now has one source of truth, retained responses use execution IDs, imports own their dialogs, writes call `sync_all`, credentials are disclosed as plaintext, and M2/M3 are split into manageable increments. 

The reqwest feature configuration is also now correct for `0.13`: `rustls` and `http2` are explicit, while proxy and cookie features are omitted.  ([Docs.rs][1])

I would make the following targeted corrections rather than rewrite the plan again.

## Must fix

### 1. Correct the clipboard capability typo

The plan uses two different names:

```text
clipboard-manager:allow-write-text
clipboard-manager:allow-text-write
```

The correct permission is:

```text
clipboard-manager:allow-write-text
```

Fix line 223 so it agrees with the Versions and Security sections. The official Tauri permission table only defines `allow-write-text`.   ([Tauri][2])

### 2. Apply authentication before URL serialization

The current pipeline says:

```text
resolve variables
→ serialize RequestUrl
→ apply auth
```

That is incorrect for API-key authentication placed in the query string. Once `RequestUrl` has been serialized, applying query authentication either requires reparsing the URL or bypassing the canonical URL model. 

Use:

```text
clone draft
→ build variable scope
→ resolve draft fields, including auth fields
→ apply auth to structured headers and RequestUrl.query
→ serialize body
→ infer Content-Type
→ serialize RequestUrl exactly once
→ validate final transport request
```

Also define authentication conflicts now:

```text
Enabled manual Authorization header
→ manual header wins
→ configured auth is skipped
→ warning is returned

Existing enabled API-key header/query parameter
→ manual value wins
→ configured API-key auth is skipped
→ warning is returned
```

The test plan already mentions auth conflicts, but the expected behavior is not specified. 

### 3. Make proxy exclusion explicit in client construction

Disabling reqwest’s `system-proxy` feature is helpful, but Cargo features are additive across the dependency graph. To guarantee that request-kit never honors environment or system proxies in v0.1, call:

```rust
Client::builder()
    .no_proxy()
```

on both reusable clients. Reqwest documents `no_proxy()` as the explicit way to disable automatic proxy usage.  ([Docs.rs][3])

Change the client requirement to:

```text
Both clients use ClientBuilder::no_proxy().
```

### 4. Redesign the import command boundary

The command surface says:

```text
choose_and_import_workspace() → parsed text
```

but the import design says the backend validates, backs up, stages, replaces, and rolls back the library. No command is defined to apply the parsed import after the user confirms replacement.  

Use a two-step backend-owned flow:

```ts
inspect_workspace_import(): Promise<ImportPreview>

interface ImportPreview {
  importId: string;
  collections: number;
  requests: number;
  environments: number;
  warnings: ImportWarning[];
  containsCredentials: boolean;
}

apply_workspace_import(importId: string): Promise<WorkspaceBootstrap>
cancel_workspace_import(importId: string): Promise<void>
```

The first command opens the dialog, validates the file, and stages it. The frontend only receives an opaque `importId` and summary. After confirmation, the second command performs the replacement.

For export:

```text
choose_and_export_workspace({ includeSecrets })
```

should build the export from backend storage. Avoid passing an entire workspace payload from the frontend because it could be stale, partially edited, or inconsistently redacted.

### 5. Make replacement import crash-recoverable

The plan handles ordinary failures but not a process or machine crash between renaming the current library and installing the staged library.

A directory replacement is not one indivisible operation across macOS and Windows. Add a transaction marker:

```text
~/.request-kit-import-transaction.json
~/.request-kit.import-staging-<id>
~/.request-kit.backup-<timestamp>
```

Flow:

```text
1. Validate and completely stage the import.
2. Write and fsync the transaction marker.
3. Rename current library to backup.
4. Rename staged library to ~/.request-kit.
5. Reload and verify the replacement.
6. Clear the marker.
7. Retain or remove the backup according to policy.
```

At startup, detect an unfinished transaction and recover either the staged library or the backup. The current rollback test only covers a returned error, not interruption between filesystem operations.  

### 6. Complete the response-retention lifecycle

Keying retained bodies by execution ID fixes the race, but releasing only on tab close is incomplete. Older and stale responses can remain in the FIFO even though nothing in the interface can save them.  

Define these rules:

```text
Stale completion received
→ ignore UI result
→ immediately release_response(staleExecutionId)

New completion accepted
→ release the previously displayed execution
→ retain the new execution

Tab closed
→ cancel in-flight execution
→ release displayed execution

Send fails or is cancelled
→ retain no response bytes
```

Also make retention insertion and FIFO eviction one synchronized operation so concurrent completions cannot exceed either budget.

### 7. Remove the claim that response headers are ordered

`HttpResponseData` currently describes “ordered header pairs.” Reqwest exposes headers through `HeaderMap`, whose iteration order is explicitly arbitrary. Duplicate values can be preserved, but original wire order cannot.  ([Docs.rs][4])

Change it to:

```text
responseHeaders: header pairs with duplicate values preserved;
global ordering is unspecified
```

Similarly, `statusText` should be described as the canonical label for the status code, not necessarily the server’s original HTTP/1 reason phrase.

### 8. Redact authentication fields from exports and cURL

The export rules currently exclude only variables marked `secret`. However, exported requests can contain:

* Basic-auth passwords
* Literal bearer tokens
* Literal API-key values

Those values are credentials even when they are not backed by a secret variable. 

Default export should blank:

```text
basic.password
bearer.token
apikey.value
all secret variable values
```

The same `includeSecrets` confirmation should restore all of them.

Raw request bodies may also contain credentials that request-kit cannot identify reliably. The export confirmation should state that request bodies are included unchanged.

The cURL behavior has the same issue. `resolveVariables: false` does not protect a bearer token entered literally. Use:

```text
Copy cURL
→ unresolved template variables
→ literal auth credentials replaced with <redacted>

Copy cURL with credentials
→ explicit warning
→ resolved variables and literal credentials included
```

The M3b replay test should explicitly use **Copy cURL with credentials**. 

## Recommended clarifications

### URL variables containing query strings

The `RequestUrl.base` field is defined as the portion before query and fragment. Define what happens when:

```text
{{baseUrl}} = https://example.com/api?tenant=123
```

and the request also has query rows. Without a rule, serialization can generate two question marks.

For v0.1, enforce:

```text
Variables used in RequestUrl.base may resolve to scheme, authority, and path,
but must not introduce ? or #.
```

Users should place query values in parameter rows.

### URL reconciliation needs an exact algorithm

“Reconcile by key + position” is still ambiguous for duplicates:

```text
?a=1&a=2
```

Define a deterministic approach:

1. Parse enabled rows from the URL bar.
2. Reuse existing enabled row IDs by ordinal position.
3. Update reused rows with parsed key, value, and `hasEquals`.
4. Create IDs for additional parsed rows.
5. Remove unmatched enabled rows.
6. Preserve disabled rows in their existing relative positions.

This prevents descriptions and row identity from moving unpredictably.

### Content-Type milestone wording

M1 already includes `infer Content-Type`, while M4 lists “automatic Content-Type” as a new feature.  

Clarify:

```text
M1:
- JSON → application/json when absent
- raw text → no automatic Content-Type

M4:
- form-urlencoded
- GraphQL
- multipart boundary handling
- binary-body behavior
```

### Test-server orchestration

The plan states that ordinary `cargo test` exercises timeout and redirect behavior against the Bun fixture server, but it does not say how that server starts or how its port is discovered. 

Either:

```text
cargo test
→ Rust integration tests start an in-process fixture server on port 0
```

or:

```text
bun run test:all
→ start fixture server on port 0
→ export its URL
→ run cargo test
→ run Vitest
→ stop fixture server
```

The first option is more reliable for Rust tests. A small raw `TcpListener` fixture is also better suited to abrupt-close and malformed-response cases.

### Duplicate UUID selection must be deterministic

“The later-scanned file is quarantined” depends on filesystem iteration order. Sort all discovered relative paths before UUID reconciliation, or define the lexicographically first path as authoritative. Otherwise different machines could quarantine different copies. 

### Define the cURL shell dialect

A generated cURL command that works in zsh or Bash is not necessarily valid in PowerShell or `cmd.exe`.

For v0.1, declare one target:

```text
Generated cURL dialect: POSIX shell
Supported terminals: Bash, zsh, Git Bash, WSL
```

Adding native PowerShell output can be a later generator.

### Remove the undefined quick-open shortcut

The hotkey registry includes:

```text
mod+P quick-open
```

but no quick-open component or M3 feature is defined. 

Either defer `mod+P` to M5 with the command palette or define a separate quick-open overlay in M3. Deferring it is leaner.

### Clarify what “duplicate” means in M2a

M2a says collections and nested folders support create, rename, delete, and duplicate, while the Rust command surface only defines `duplicate_request`.  

For v0.1, state:

```text
Duplicate applies to requests only.
Folder and collection duplication is deferred.
```

Recursive folder duplication is a meaningful amount of additional storage and conflict-handling work.

## Storage warning

Because request files can contain unencrypted bearer tokens, API keys, basic-auth passwords, and body credentials, `~/.request-kit` should be described as **human-readable**, but not generally **safe to commit to Git**.  

Add:

```text
Unix:
- storage directories created with mode 0700
- files containing user data created with mode 0600

Windows:
- inherited per-user ACLs are used

The app-managed ~/.request-kit directory should not be committed to Git.
Sanitized export is the supported version-control path for v0.1.
```

Also define backup retention because replacement backups contain the same plaintext secrets—for example, keep only the latest three backups and allow users to delete them from Settings.

## Final assessment

After the eight must-fix items, the plan is ready for M0. The remaining items are specification clarifications that can be added without changing architecture or scope. A fourth full planning rewrite is unnecessary; a focused Revision 3.1 patch would be sufficient.

[1]: https://docs.rs/crate/reqwest/latest/features?utm_source=chatgpt.com "reqwest 0.13.4 - Docs.rs"
[2]: https://v2.tauri.app/plugin/clipboard/?utm_source=chatgpt.com "Clipboard | Tauri"
[3]: https://docs.rs/reqwest/latest/reqwest/?utm_source=chatgpt.com "reqwest - Rust"
[4]: https://docs.rs/http/latest/http/header/struct.HeaderMap.html?utm_source=chatgpt.com "HeaderMap in http::header - Rust"

