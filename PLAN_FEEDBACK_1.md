## Review

The plan is technically strong, but it is **not yet lean**. It describes a polished v1 rather than a minimal first release. The largest scope drivers are nested collection storage, path-based tab state, environment inheritance, a hand-written cURL parser, Postman import, multipart streaming, response caching, HTML preview, GraphQL tooling, code generation, and the command palette. Together, those features dominate the implementation risk even though the document labels them as MVP work. 

The core stack is appropriate. The listed Tauri `2.11.x`, plugin versions, reqwest `0.13.x`, Vue `3.5.x`, and Vite `8.1.x` are consistent with current stable releases. Tauri is currently in the `2.11` series, Vite `8.1` is supported, reqwest is at `0.13.4`, and Vue `3.5` remains stable while `3.6` is still prerelease. ([Tauri][1])

I would make the following changes before implementation.

### 1. Define a genuinely small v0.1

Keep:

* Standard HTTP methods
* URL and query parameters
* Headers
* None, raw text, and JSON bodies
* Send and cancel
* Timeout and redirect toggle
* Status, duration, headers, raw text, and formatted JSON responses
* Save, rename, duplicate, and delete requests
* One-level collections
* Basic history
* Light and dark themes
* Basic, bearer, and API-key authentication
* One environment with `{{variables}}`
* Copy as cURL
* macOS and Windows installers

Defer:

* Nested folders and drag-and-drop ordering
* Folder and collection variable inheritance
* Request-scoped variables
* Multipart and binary bodies
* GraphQL editor
* Postman import
* General cURL import
* `wget` generation
* HTML preview
* Large-response cache
* Command palette
* Drag-and-drop file import
* Persistent unsaved tabs
* Detailed TLS/DNS error taxonomy

### 2. Reuse reqwest clients

The proposed fresh `reqwest::Client` for every send should be changed. Reqwest explicitly recommends reusing clients because each client owns a connection pool. ([Docs.rs][2])

For the lean implementation:

```text
AppState
├── follow_redirects_client
└── no_redirects_client
```

Use the request-level timeout API rather than rebuilding a client for each timeout. Keep one global maximum redirect count initially. Reqwest supports client-level redirect policy and request-level timeout configuration. ([Docs.rs][3])

Also decide whether OS proxy behavior is intentional. Reqwest `0.13` enables `system-proxy` in its default feature set. If proxy support is truly out of scope, disable default features and explicitly enable only what you need. ([Docs.rs][4])

### 3. Remove the response LRU from the first release

The bounded 100 MB response cache, execution IDs, cache eviction handling, full-response save, IPC preview truncation, and download caps create a subsystem of their own. 

For v0.1:

* Hard-cap the downloaded body at 10 MB.
* Return at most 2 MB to the frontend.
* Allow saving only retained responses.
* Clearly show when a response exceeded the cap.
* Add temporary-file spooling later if users actually need large downloads.

Also remove `headerBytes` unless it is explicitly labelled an estimate. Avoid claiming wire-accurate response size or timing; report decoded body bytes and application-observed elapsed time.

### 4. Add one canonical request preparation pipeline

The current design spreads request transformation across variable resolution, query synchronization, authentication, content-type logic, body components, sending, and code generation. This will produce subtle differences between “Send” and “Copy as cURL.”

Add one pure TypeScript function:

```ts
prepareRequest(
  draft: RequestDraft,
  context: RequestContext,
): PreparedRequestResult
```

Its order should be:

```text
1. Clone the draft
2. Merge enabled query rows
3. Build the variable scope
4. Resolve variables
5. Apply authentication
6. Serialize the selected body
7. Infer Content-Type when absent
8. Validate URL, headers, and body
9. Return a normalized transport request
```

Use the same result for:

* Sending
* cURL generation
* Request preview
* Validation
* Future code generation

### 5. Simplify variables

The proposed precedence chain is useful but belongs after the first release. 

Start with:

```text
active environment > global
```

Then add collection, folder, and request scopes after the core app is stable.

When recursive variable values are added, specify the behavior precisely:

* Maximum recursion depth
* Cycle detection
* Unresolved-variable errors
* Disabled-variable handling
* Whether variable keys are case-sensitive
* Whether variables are resolved in header names as well as values

The current wording “one level of nesting, depth-limit 5” is contradictory.

### 6. Prevent secrets from leaking into history and exports

Secrets are only masked visually, while the history currently stores the resolved URL. That means an API key inserted into a query parameter could be written to `history.jsonl` in plaintext. 

Use these rules:

* Store the template URL in history when possible.
* Redact known API-key query parameters.
* Never log authorization headers.
* Default cURL generation to unresolved/template values.
* Require an explicit reveal action before copying resolved secrets.
* Exports should exclude secrets by default, with an opt-in confirmation.

### 7. Separate app data from user-owned projects

`~/.request-kit` works, but it is neither a conventional desktop-app data location nor a true user-selectable Bruno-style project. The plan currently mixes those two models. 

For the lean release, use Tauri’s application data path:

```rust
app.path().app_data_dir()
```

Tauri provides this specifically as the suggested platform-specific application data location. ([Docs.rs][5])

Later, add “Open project folder” as a separate feature for Git-friendly collections.

### 8. Avoid paths as frontend identity

The plan uses relative file paths to identify requests and stores those paths in tabs, UI state, and history. Renaming or moving a request then requires updating every reference. 

Prefer:

```ts
type RequestReference = {
  id: string;
  path: string;
}
```

Use the UUID as frontend identity and the path as backend storage metadata. After a rename or move, only the path map changes.

For v0.1, avoiding nested moves removes most of this complexity.

### 9. Add single-instance protection

Atomic files do not prevent two running app processes from reading the same state and overwriting each other’s changes. Add the Tauri single-instance plugin or an equivalent process lock before persistence work. A maintained Tauri single-instance plugin is available in the current plugin ecosystem. ([Tauri][6])

### 10. Replace external smoke-test dependencies

Do not make `httpbin.org` the primary acceptance environment. Add a small local test server supporting:

* Delayed response
* Redirect chain
* JSON echo
* Duplicate headers
* Gzip response
* Binary response
* Configurable response size
* Multipart upload, when implemented
* Abrupt connection close

External services can remain in the manual checklist, but deterministic tests should run locally.

### 11. Simplify error classification

The proposed DNS, certificate, handshake, refusal, reset, and other detailed classifications will vary by OS, resolver, and error source chain. String matching DNS messages is especially fragile. 

Start with:

```text
invalidUrl
cancelled
timeout
tls
network
io
unknown
```

Return the original safe detail for diagnostics. Add more specific categories only when you have fixtures demonstrating consistent macOS and Windows behavior.

### 12. Move release builds and CI earlier

Installer creation should not wait until M6. Build both targets as soon as the first request can be sent. Platform packaging, permissions, WebView2, window restoration, and file paths are foundational risks.

The GitHub Actions matrix should be part of foundation work, even if code signing and publishing remain later.

## Revised implementation plan

# request-kit — Lean Implementation Plan

## Product definition

request-kit is a single-user, local-first desktop REST client for macOS and Windows.

The first release focuses on composing, sending, inspecting, and saving ordinary HTTP requests. It does not attempt broad Postman compatibility.

## Explicit v0.1 scope

### Requests

* GET, POST, PUT, PATCH, DELETE, HEAD, and OPTIONS
* URL input
* Query parameter rows
* Header rows
* Body modes:

  * None
  * Raw text
  * JSON
* Send and cancel
* Per-request timeout
* Follow-redirects toggle
* Basic, bearer-token, and API-key authentication

### Responses

* Status code
* Standard status label when available
* Final URL
* HTTP version
* Elapsed duration
* Decoded body size
* Response headers
* Raw text view
* Formatted JSON view
* Copy response body
* Hard body limit with a visible truncation warning

### Persistence

* One application-managed library
* One-level collections
* Save request
* Rename request
* Duplicate request
* Delete request
* Recent request history
* Search by request name, URL, and method

### Variables

* Global variables
* One selected environment
* `{{variable}}` substitution
* Unresolved-variable warnings
* Secret masking in the interface

### Desktop behavior

* Light, dark, and system themes
* Window state restoration
* Cmd/Ctrl+Enter to send
* Cmd/Ctrl+S to save
* Cmd/Ctrl+N to create a request
* macOS application and DMG
* Windows NSIS installer

### Interchange

* Copy current request as cURL
* Export and import request-kit’s own JSON format

## Deferred features

* Nested folders
* Manual tree ordering
* Moving nodes
* Multiple workspaces
* User-selected Git project folders
* Multipart form data
* Binary request bodies
* GraphQL editor (GraphQL is out of scope)
* HTML preview
* cURL command import
* Postman import
* Drag-and-drop import
* Request, folder, and collection variable scopes
* Collection-level inherited authentication
* Large-response disk spooling
* Command palette
* wget and language code generation
* Persistent unsaved drafts
* Cookies, proxy configuration, TLS overrides, scripting, tests, WebSocket, gRPC, and accounts

## Technical stack

* Tauri 2
* Rust backend
* Vue 3
* TypeScript
* Vite
* Bun
* Pinia
* CodeMirror 6
* reqwest
* Tokio
* serde and serde_json
* thiserror
* uuid
* tempfile for safe replacement writes where appropriate

All package versions are pinned by the lockfiles. The plan records major or minor compatibility ranges rather than hard-coding patch versions.

## Architecture

```text
Vue frontend
├── Request editor
├── Response viewer
├── Collections and history
├── Environment editor
└── Settings

Pure TypeScript domain layer
├── prepareRequest.ts
├── variable resolution
├── URL/query synchronization
├── JSON formatting
└── cURL generation

Typed IPC boundary
└── src/ipc/commands.ts

Rust backend
├── HTTP executor
├── cancellation registry
├── application storage
├── import/export
└── error normalization
```

The frontend must not import Tauri APIs outside the IPC wrapper and application bootstrap modules.

## Request preparation

One pure function produces the normalized request used by sending and cURL generation.

```text
draft
→ query merge
→ variable resolution
→ authentication
→ body serialization
→ Content-Type inference
→ validation
→ transport payload
```

The function returns either a prepared request or structured validation errors and warnings.

## HTTP backend

### Shared clients

Maintain two reusable reqwest clients:

* Redirect-following client
* No-redirect client

Per-request timeout is applied to the request itself.

The following are disabled or omitted in v0.1:

* Cookie store
* Invalid-certificate acceptance
* Custom proxy configuration
* Automatic retries initiated by request-kit

Whether reqwest may use the operating-system proxy must be an explicit dependency configuration decision.

### Cancellation

Each send receives an execution UUID.

```rust
Mutex<HashMap<ExecutionId, CancellationToken>>
```

The executor races request execution against the cancellation token and removes the token with an RAII guard.

### Response limits

Initial limits:

```text
Maximum downloaded body: 10 MB
Maximum body sent over IPC: 2 MB
Maximum response cache: none
```

When the body exceeds the download cap, request-kit stops reading and marks the response as capped.

Saving bodies larger than the retained data is deferred until disk spooling is implemented.

### Error model

```ts
type ErrorKind =
  | "invalidUrl"
  | "cancelled"
  | "timeout"
  | "tls"
  | "network"
  | "io"
  | "unknown";
```

Non-2xx HTTP responses are normal responses, not application errors.

## Storage

Use the Tauri application data directory.

```text
<app-data>/request-kit/
├── settings.json
├── ui-state.json
├── globals.json
├── environments/
├── collections/
└── history.jsonl
```

### Initial collection layout

```text
collections/
└── <collection-id>/
    ├── collection.json
    └── requests/
        └── <request-id>.json
```

File names use UUIDs. User-facing names remain inside JSON and therefore do not require slug-based renaming.

This removes most filename collision and Windows reserved-name handling.

### File identity

UUID is the stable identity.

Paths are storage details and are never the primary identity in frontend state, tabs, or history.

### Writes

* Write a temporary file in the destination directory.
* Flush file contents.
* Atomically replace the target where supported.
* Preserve the previous file when replacement fails.
* Reject symbolic links inside app-managed collection storage.
* Recover from individual corrupt request files without preventing the entire app from starting.

Schema version fields are included, but migration infrastructure is added only when the first schema change occurs.

### History

History stores:

* Execution time
* Method
* Template URL or redacted URL
* Status
* Duration
* Request UUID
* Error category

Authorization headers and resolved secrets are never stored.

History compaction occurs at startup or after a size threshold rather than rewriting the file after every request.

## Frontend state

Use four primary stores:

```text
library
tabs
settings
environment
```

History may remain inside the library store initially.

Dirty state is explicit. Editing a request marks its tab dirty; saving clears it. Avoid continuous deep comparison of large request bodies.

Only saved tabs are restored in v0.1. Unsaved scratch requests display a warning before closing or quitting.

## URL and query synchronization

The URL string remains the source of truth.

Parsing must preserve:

* Duplicate keys
* Empty values
* Keys without values
* URL fragments
* Percent-encoded content
* Template variables
* Values containing encoded delimiters

Editing parameter rows replaces only the query portion of the URL.

Parsing failures must not destroy the original URL.

## Variable behavior

Initial precedence:

```text
environment > global
```

Rules:

* Keys are case-sensitive.
* Disabled variables do not resolve.
* Recursive values have a maximum depth of five.
* Cycles produce a validation error.
* Unresolved variables block sending by default.
* Users may explicitly send with unresolved variables.
* Secret values are masked in editors, previews, history, logs, and exports.

## Security boundaries

* No remote web content is injected into the application DOM.
* HTML preview is deferred.
* Custom Rust file commands operate only inside app data.
* Import and export use native file dialogs and size limits.
* Tauri capabilities expose only required APIs.
* Only one application instance may modify storage.
* Response and request content is never written to logs by default.

## Milestones

### M0 — Foundation

* Initialize Git
* Scaffold Tauri, Vue, TypeScript, Vite, and Bun
* Configure linting, formatting, Rust tests, and Vitest
* Add macOS and Windows CI build jobs
* Add single-instance handling
* Resolve the application data directory
* Define TypeScript and Rust IPC types
* Add a deterministic local HTTP fixture server
* Produce an empty macOS and Windows application build

Exit condition: both platform builds launch and CI passes.

### M1 — Send one request

* Request method and URL
* Query and header editors
* None, raw, and JSON bodies
* Request preparation pipeline
* Shared reqwest clients
* Send, cancel, timeout, and redirect toggle
* Raw and formatted JSON response views
* Status, headers, duration, size, and final URL
* Response body limits
* Broad error categories

Exit condition: the local fixture suite covers JSON, delay, cancellation, redirect, large body, malformed URL, and network failure.

### M2 — Save and reopen

* Application storage
* One-level collections
* Request save, rename, duplicate, and delete
* Stable UUID identities
* Dirty state
* History
* Search
* Settings
* Theme and window state
* Corrupt-file recovery
* Release installers for manual testing

Exit condition: create, save, quit, reopen, rename, and delete work on macOS and Windows without data loss.

### M3 — Daily-use features

* Multiple tabs
* Global and environment variables
* Variable validation and masking
* Basic, bearer, and API-key authentication
* Copy as cURL
* Own-format import and export
* Keyboard shortcuts
* Response body copy

Exit condition: a saved request using environment variables and authentication can be reopened, sent, and reproduced with generated cURL.

### M4 — Optional expansion

Features enter this milestone individually and require a demonstrated use case:

* Multipart uploads
* Binary requests
* Large-response disk spooling
* cURL import
* Postman import
* Nested folders
* Inherited variables and authentication
* GraphQL editor
* HTML preview with restrictive content security policy
* Command palette
* Additional code generators

Each feature receives its own schema, tests, and acceptance checklist rather than being implemented as one combined milestone.

## Testing

### Rust

* Request cancellation cleanup
* Timeout behavior
* Redirect behavior
* Body cap
* Broad error mapping
* Atomic replacement
* Corrupt-file recovery
* UUID identity
* History redaction
* Symlink rejection

### TypeScript

* Request preparation ordering
* URL/query synchronization
* Duplicate query parameters
* Variable precedence
* Recursive variables and cycles
* Authentication conflicts
* Content-Type inference
* cURL quoting
* Dirty-state transitions

### Component tests

Use mocked IPC for:

* Sending and cancellation
* Saving and dirty state
* Closing dirty tabs
* Query editor synchronization
* Environment switching

### Manual smoke tests

Run installers on both target operating systems at every release candidate, not only after all features are complete.

## Release definition

v0.1 is complete when a user can:

1. Create an HTTP request.
2. Add query parameters, headers, authentication, and a JSON body.
3. Send or cancel it.
4. Inspect status, headers, timing, and formatted response data.
5. Save it into a collection.
6. Reopen it after restarting the application.
7. switch environments.
8. Copy an equivalent cURL command.
9. Install and run the application on macOS and Windows.

The main product decision still worth settling is whether collections are permanently app-managed or should eventually live in user-selected Git folders. The revised plan assumes app-managed storage for v0.1 and treats Git folders as a later workspace feature.

[1]: https://v2.tauri.app/release/?utm_source=chatgpt.com "Tauri Core Ecosystem Releases | Tauri"
[2]: https://docs.rs/reqwest/latest/reqwest/struct.Client.html?utm_source=chatgpt.com "Client in reqwest - Rust"
[3]: https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html?utm_source=chatgpt.com "ClientBuilder in reqwest - Rust"
[4]: https://docs.rs/crate/reqwest/latest/features?utm_source=chatgpt.com "reqwest 0.13.4 - Docs.rs"
[5]: https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html?utm_source=chatgpt.com "PathResolver in tauri::path - Rust"
[6]: https://tauri.app/release/?utm_source=chatgpt.com "Tauri Ecosystem Releases | Tauri Releases"
