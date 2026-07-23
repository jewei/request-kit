## Overall assessment

Revision 2 is substantially better. It has addressed the major architectural risks from the previous review: reusable HTTP clients, stable UUID identity, canonical request preparation, staged variables, redacted history, deterministic fixtures, single-instance protection, and earlier platform builds. The M0–M3 definition also creates a clearer v0.1 boundary. 

I would proceed after correcting the following issues.

## Must fix before implementation

### 1. The reqwest feature name is incorrect

The plan specifies:

```toml
features = ["rustls-tls", "json", "multipart", "stream", "gzip", "brotli", "deflate"]
```

For reqwest `0.13`, the feature is named **`rustls`**, not `rustls-tls`. Also, `default-features = false` disables HTTP/2, charset decoding, and system proxy support. HTTP/2 must be added explicitly if you want it. Reqwest `0.13.4` also requires Rust `1.85.0` or newer.  ([Docs.rs][1])

A better v0.1 configuration is:

```toml
reqwest = {
  version = "0.13",
  default-features = false,
  features = [
    "rustls",
    "http2",
    "stream",
    "gzip",
    "brotli",
    "deflate"
  ]
}
```

Add `multipart` in M4. `json` is unnecessary if the frontend always serializes JSON into text. Add `charset` only if responses encoded as something other than UTF-8 should render as text.

Also add:

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.85"
components = ["rustfmt", "clippy"]
```

Using a later pinned stable version is also reasonable.

### 2. URL and query parameters still have two sources of truth

The plan says:

* The URL string is the source of truth.
* `RequestFile` stores both `url` and `params[]`.
* `prepareRequest` merges enabled parameter rows into the URL.

Those rules conflict. Disabled parameter rows cannot be represented in the URL, and enabled rows could be duplicated if they already exist in both places.  

Use a canonical model such as:

```ts
interface RequestUrl {
  base: string;
  query: QueryParam[];
  fragment: string;
}

interface QueryParam {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
  hasEquals: boolean;
}
```

The URL bar becomes an editable projection:

```text
RequestUrl model → displayed URL
Displayed URL edit → parse back into RequestUrl model
prepareRequest → serialize RequestUrl exactly once
```

This preserves disabled parameters, descriptions, duplicate keys, keys without `=`, and fragments without maintaining two competing representations.

### 3. `maxRedirects` is inconsistent

`prepareRequest` receives `maxRedirects`, but the reusable redirect client is fixed at ten redirects. The transport payload and settings schema no longer include `maxRedirects`.  

For v0.1, remove `maxRedirects` from `RequestContext` entirely:

```ts
appDefaults: {
  timeoutMs: number;
  followRedirects: boolean;
  maxBodyBytes: number;
}
```

Keep the redirect limit fixed at ten. Arbitrary per-request redirect limits would require additional clients or a different redirect architecture because reqwest redirect policy belongs to the client.

### 4. Retaining responses by `tabId` creates a race

The proposed retention map is:

```rust
HashMap<TabId, Bytes>
```

and saving uses:

```text
save_response_body(tabId, path)
```

A cancelled or slower earlier request could finish after a newer request and overwrite the retained bytes for that tab. Saving could then write a body that does not match the response currently displayed. 

Key retained responses by execution ID:

```rust
struct RetainedResponse {
    execution_id: ExecutionId,
    tab_id: TabId,
    bytes: Bytes,
}

HashMap<ExecutionId, RetainedResponse>
```

Then use:

```text
save_response_body(executionId)
release_response(executionId)
```

The frontend should ignore any completion whose `executionId` is not the tab’s latest execution.

The memory bound also needs a global limit. Ten megabytes multiplied by an unlimited number of tabs is not naturally bounded. A simple FIFO budget is enough:

```text
Maximum retained bytes: 50 MB
Maximum retained responses: 10
```

This does not require a sophisticated LRU implementation.

### 5. Atomic writes need `sync_all`, not only `flush`

The revised plan changed the write sequence to temporary file → flush → rename. `flush()` only pushes userspace buffers toward the operating system. The `tempfile` documentation explicitly notes that `persist()` does not synchronize the file contents or containing directory.  ([Docs.rs][2])

Use:

```rust
let mut temp = tempfile::NamedTempFile::new_in(parent)?;
serde_json::to_writer_pretty(temp.as_file_mut(), value)?;

temp.as_file_mut().flush()?;
temp.as_file().sync_all()?;

temp.persist(target)?;
```

On Unix-like systems, optionally sync the parent directory after persistence for stronger crash durability. This can be best-effort on Windows.

### 6. Import/export paths are not actually proven to be dialog-selected

The commands currently accept arbitrary paths:

```text
read_import_file(path)
write_export_file(path)
save_response_body(tabId, path)
```

The security section says those paths are dialog-selected, but the Rust backend cannot verify that a frontend-provided path came from a dialog. A save destination may also not exist yet, so it cannot be fully canonicalized.  

Prefer commands that own the dialog and file operation:

```text
choose_and_import_workspace()
choose_and_export_workspace(payload)
choose_and_save_response(executionId)
```

The Rust side opens the native dialog and immediately reads or writes the selected path. This removes arbitrary file paths from the IPC contract.

Symlink and traversal rejection should remain strict inside `~/.request-kit`. It is less useful for a path the user explicitly selects outside app-managed storage.

### 7. Secret handling needs two additions

The plan states that variable values are stored in plaintext, but request authentication values will also be plaintext:

* Basic-auth passwords
* Bearer tokens
* API keys
* Possibly raw body credentials

That should be disclosed alongside the variable warning. 

Also, this statement is unsafe:

> Original error chain always preserved in `detail`.

A resolved URL containing an API key could appear in the reqwest error chain. Redact URL query values and credentials before returning `AppError.detail`. Keep the original chain only in memory or in debug builds with explicit redaction. 

## Recommended corrections

### Response headers after decompression

With reqwest automatic gzip, Brotli, or deflate decoding enabled, reqwest removes `Content-Encoding` and `Content-Length` from the response headers and returns the decoded body. ([Docs.rs][3])

Therefore, the UI should not imply that it displays wire-original headers. Label the values internally and in documentation as:

```text
Decoded body size
Normalized response headers
```

Alternatively, retaining original headers requires disabling automatic decoding and implementing decoding separately, which is unnecessary for v0.1.

### Clipboard support is missing from the dependency plan

M1 and M3 require copying response bodies and cURL commands, but no clipboard implementation is specified. Either explicitly test `navigator.clipboard.writeText` on both WebViews or add Tauri’s clipboard-manager plugin with only:

```text
clipboard-manager:allow-write-text
```

The official plugin enables no clipboard permissions by default. ([Tauri][4])

### Register the single-instance plugin first

Tauri’s documentation says the single-instance plugin should be registered before other plugins so it runs before they can interfere. Add that requirement to M0. ([Tauri][5])

The second-instance callback should:

```text
show window
unminimize window
focus window
```

### Define import behavior before M3

“Export → wipe → import round-trips” does not define whether importing:

* Replaces the full library
* Merges collections
* Regenerates IDs
* Overwrites matching IDs
* Renames collisions
* Restores environments and globals

For the leanest behavior, make v0.1 import a full replacement:

```text
1. Parse and validate the complete import.
2. Show a replacement confirmation.
3. Create a timestamped backup.
4. Stage the imported library separately.
5. Replace the current library only after staging succeeds.
6. Restore the backup if replacement fails.
```

Merge import can arrive later.

### Handle duplicate UUIDs

Human-readable storage means users may copy request files manually. That can create two files with the same embedded UUID, breaking the `id → path` map.

Define scan behavior now:

```text
Duplicate UUID
→ do not silently choose one
→ quarantine the later file
→ show its original location and recovery action
```

Also test case-only renames such as `Users` → `users` on case-insensitive Windows and macOS filesystems.

### Strengthen the future HTML preview

An empty iframe `sandbox` prevents script execution and many capabilities, but it does not itself define which external resources the document may fetch. Add a restrictive CSP to the `srcdoc`, for example:

```html
<meta
  http-equiv="Content-Security-Policy"
  content="
    default-src 'none';
    img-src data: blob:;
    style-src 'unsafe-inline';
    font-src data:;
  "
>
```

CSP can be used to restrict the sources from which scripts and other resources are fetched inside `srcdoc`. ([MDN Web Docs][6])

## Milestone adjustment

The scope is now staged, but M2 and M3 remain very large. 

I would split them internally without changing the public release scope:

```text
M2a — Storage and saved requests
M2b — History, search, settings, themes, window state

M3a — Tabs, variables, and authentication
M3b — cURL, import/export, hotkeys, and editor polish
```

Also change the M0 exit condition from:

```text
Installers build in CI and launch
```

to:

```text
Installers build in CI.
Unsigned installers are launched manually on both target systems.
```

A hosted build job can verify packaging, but it is not a reliable GUI-launch test.

## Verdict

The plan is now coherent enough to begin once the following five corrections are made:

1. Fix reqwest features and pin the Rust toolchain.
2. Resolve the URL/parameter source-of-truth conflict.
3. Remove the unused `maxRedirects` setting.
4. Key retained responses by execution ID with a global memory budget.
5. Move dialog-selected file operations fully behind Rust commands.

The remaining recommendations can be incorporated while implementing M0–M3.

[1]: https://docs.rs/crate/reqwest/latest/features?utm_source=chatgpt.com "reqwest 0.13.4 - Docs.rs"
[2]: https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html?utm_source=chatgpt.com "NamedTempFile in tempfile - Rust"
[3]: https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html?utm_source=chatgpt.com "ClientBuilder in reqwest - Rust"
[4]: https://v2.tauri.app/plugin/clipboard/?utm_source=chatgpt.com "Clipboard | Tauri"
[5]: https://v2.tauri.app/fr/plugin/single-instance/?utm_source=chatgpt.com "Single Instance | Tauri"
[6]: https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc?utm_source=chatgpt.com "HTMLIFrameElement: srcdoc property - Web APIs | MDN"
