# M2a — Storage and saved requests (design)

Milestone M2a from `PLAN.md`. This spec resolves the open design decisions and
fixes the module boundaries; the implementation plan is derived from it.

Approved decisions (brainstorm, 2026-07-23):

- **Partial bootstrap.** `load_workspace` implements/persists only
  collections + folders + requests. It returns safe defaults for the M2b parts
  (empty environments/globals, default settings, empty ui-state). The single
  bootstrap contract is kept; the fields are filled in during M2b.
- **Functional sidebar + single tab.** Recursive collection/folder/request tree
  with context-menu CRUD, alphabetical order, click-to-open into the current
  single tab (warn if dirty), `mod+S` save, dirty dot + close/quit warning. No
  search (M2b), no reorder/move (M4), no tab bar yet (M3a).
- **In-memory workspace index in `AppState` (Approach 1).** Disk is the single
  source of truth; the index is a rebuildable cache. Raw IO is pure free
  functions taking `&Path`; the store orchestrates and owns the index + root.

## Goals / non-goals

**Goals:** create → save → quit → relaunch → rename → delete a request with no
data loss on macOS and Windows; corrupt and duplicate-UUID files quarantine
cleanly and deterministically; all writes atomic and durable; storage
permissions `0700`/`0600` on Unix.

**Non-goals (deferred):** history, search, settings/env/ui-state persistence
(M2b); move/reorder + folder/collection duplication (M4); multi-tab (M3a);
import/export (M3b). These are named where the seams touch M2a so nothing has to
be re-cut later.

## On-disk layout (M2a subset of PLAN.md)

```
~/.request-kit/                     # 0700
└── collections/
    └── <collection-slug>/
        ├── collection.json         # { version, id, name, auth, variables }
        ├── <request-slug>.json     # RequestFile
        └── <folder-slug>/          # recursive
            ├── folder.json         # { version, id, name }
            └── <request-slug>.json
```

All files are JSON with top-level `"version": 1` and mode `0600` on Unix.
`environments/`, `globals.json`, `settings.json`, `ui-state.json`,
`history/` are **not created in M2a**; their absence yields the defaults above.

### Schema ownership (Rust envelope, frontend body)

Rust does **not** re-model the entire `RequestFile` schema (that lives in the TS
type and would drift). Instead each stored document has a typed *envelope* Rust
understands, and the rest is preserved verbatim:

```rust
struct FileHeader { version: u32, id: Uuid, name: String }   // parsed for the index
// full document round-tripped as serde_json::Value
```

- **Read/scan:** deserialize `FileHeader` to build the index; keep the full
  `Value` for `read_request`. Unsupported `version` (> 1) or a missing/invalid
  `id` ⇒ quarantine (never a hard startup failure).
- **Write:** `write_request(id, document)` receives the full `RequestFile` JSON
  from the frontend. Rust validates `document.version == 1`, `document.id == id`
  and that `id` matches the index entry (the id is immutable through writes),
  extracts `name` for the slug, and writes atomically.
- **Rename:** `rename_node(id, newName)` reads the file, sets `name`, writes to
  the new slug path, and `fs::rename`s (case-only rename handled, see below).
  References are ids, so nothing else changes.

`RequestFile` (frontend-owned TS type, forward-compatible so M3a needs no
migration): `{ version:1, id, name, method, url:RequestUrl, headers[], body,
auth:{type:'inherit'|... default 'inherit'}, variables:[] (empty in M2a),
settings }`. M2a only edits the M1 subset; `auth`/`variables` are persisted with
defaults for forward-compat.

## Backend modules (`src-tauri/src/storage/`)

Pure IO — free functions, `&Path` in, unit-tested against tempdirs:

- `paths.rs` (exists) — root resolution + `ensure_storage_root`; add
  `collections_dir(root)` and the `0700`/`0600` permission helpers.
- `atomic.rs` — `write_json_atomic(target, &Value)`: `NamedTempFile::new_in`
  (same dir) → `to_writer_pretty` → `flush` → `sync_all` → `persist(target)` →
  best-effort parent-dir fsync on Unix. Original preserved on failure. Rejects
  symlinked targets inside storage.
- `slug.rs` — `slugify(name)`: strip Windows-reserved chars `<>:"/\|?*`,
  trailing dots/spaces, reserved stems (`CON`,`PRN`,`AUX`,`NUL`,`COM1..9`,
  `LPT1..9`); empty ⇒ `untitled`; collision suffixes `-2`,`-3`. Case-only
  rename (`Users`→`users`) via two-step rename through a temp name (both target
  filesystems are case-insensitive).
- `scan.rs` — walk `collections/`, build the node tree + `id→path` index;
  quarantine corrupt/unsupported files; resolve duplicate UUIDs
  deterministically (paths normalized to `/`-separated relative form, sorted
  lexicographically, first wins, rest quarantined). Returns
  `(tree, index, Vec<QuarantineReport>)`.
- `quarantine.rs` — `quarantine(path, reason)`: rename to `<name>.corrupt-<ts>`
  (`ts` from `SystemTime`); returns a report `{ path, reason }`.
- `nodes.rs` — the node tree types (`WorkspaceNode` = collection | folder |
  request with `id, name, children`) and CRUD helpers (`create_collection`,
  `create_folder`, `create_request`, `duplicate_request`, `delete_node`) built
  on `atomic` + `slug`.

Stateful orchestration:

- `workspace.rs` — `Workspace { root, tree, index: HashMap<Uuid,PathBuf>,
  collection_meta }`; held in `AppState` as `Mutex<Workspace>`. `load()` calls
  `scan`; each mutation does the disk write then updates the index **in the same
  locked section** (single-lock invariant, mirroring `retain.rs`). Alphabetical
  (case-insensitive) ordering is applied when building the tree for the
  frontend.

`AppState` gains `pub workspace: Mutex<Option<Workspace>>` (None until first
`load_workspace`; commands that need it error `Io`/`Validation` if unloaded).

## Command surface (M2a)

All `Result<T, AppError>`, serde camelCase, registered in `lib.rs`. Rejected
with `maintenanceInProgress` when `AppMode != Normal` (scaffolding exists):

- `load_workspace() -> WorkspaceBootstrap` — scan + build index; returns
  `{ tree, environments:[], globals:[], settings:<defaults>, uiState:<defaults>,
  quarantined:[] }`.
- `create_collection(name) -> WorkspaceNode`
- `create_folder(parentId, name) -> WorkspaceNode`
- `create_request(parentId, name) -> WorkspaceNode` (writes a default
  `RequestFile`; parent may be a collection or folder)
- `read_request(id) -> RequestFile` (the stored JSON `Value`)
- `write_request(id, document) -> ()` (atomic; id immutable)
- `rename_node(id, newName) -> WorkspaceNode`
- `delete_node(id) -> ()` (recursive for folders/collections)
- `duplicate_request(id) -> WorkspaceNode` (new uuid + " copy"; **requests
  only** in v0.1)

`write_collection_meta` is deferred to M3a (nothing edits collection
auth/variables in M2a, and `rename_node` already covers collection renames).
`move_node` is M4. Import/export commands are M3b.

## Frontend

- `types/workspace.ts` — `WorkspaceNode` (`id, kind, name, children?`),
  `RequestFile`, `WorkspaceBootstrap`.
- `ipc/commands.ts` — typed wrappers for the commands above (the only place
  calling `invoke`).
- `stores/workspace.ts` — Pinia: `tree`, `id→node` lookup, `load()` on boot,
  CRUD actions delegating to IPC and applying the returned node to the tree;
  in-memory search deferred to M2b.
- Tab integration (`stores/tabs.ts`): `openRequest(id)` loads a `RequestFile`
  into the single tab (warn if dirty); `save()` becomes real —
  `write_request(requestId, toRequestFile(draft))`, clears `dirty`; a scratch
  tab's first save calls `create_request` then `write_request`. Deleting a node
  whose request is open converts the tab to a scratch tab and releases its
  retained response.
- Components: `layout/Sidebar.vue`, `sidebar/{CollectionTree,
  CollectionTreeNode}.vue`, `shared/{ContextMenu,ConfirmDialog,InlineRename}.vue`;
  `MainLayout` becomes the `sidebar | main` grid. Context-menu actions: new
  collection/folder/request, rename (inline), delete (confirm), duplicate
  (requests). Alphabetical order; dirty dot on the open request.
- `beforeunload`/close warning when the active tab is dirty (single-tab v0.1).

## Error handling

- Corrupt/unsupported/duplicate files never block startup — they quarantine and
  surface in `WorkspaceBootstrap.quarantined` for a non-blocking notice.
- Unloaded-workspace command calls, id-not-found, and version-too-new return
  typed `AppError` (`Io`/`Validation`), never panics.
- Atomic write failure preserves the original file; the command surfaces `Io`.
- All storage mutations rejected during `ImportApplying`/`Recovery`
  (`maintenanceInProgress`).

## Testing

**Rust (`cargo test`, tempdirs):** atomic replacement (no temp residue;
original survives a simulated persist failure; `sync_all` invoked via the
abstraction); slugify + Windows reserved names + collision suffixes + case-only
rename; scan builds tree + index; corrupt-file quarantine; duplicate-UUID
quarantine with lexicographic determinism (same file wins regardless of walk
order); request create→read→write round-trip; UUID identity preserved through
rename; recursive delete; duplicate = requests only; version-too-new rejected.

**TS (Vitest, mocked `ipc/commands`):** workspace store load + CRUD tree
updates; `openRequest` warns on dirty; save round-trip (scratch → create →
write, dirty cleared); delete-open-request → scratch + `releaseResponse`;
CollectionTreeNode renders nested tree + emits context-menu actions.

**Manual smoke (`docs/smoke.md` M2a):** create collection/folder/request →
save → quit → relaunch (restored) → rename → delete; hand-drop a corrupt JSON
and a duplicate-id file → both quarantine with a notice, app still starts.

## Milestone boundary check

Exit = create/save/quit/relaunch/rename/delete with no data loss on both OSes +
deterministic quarantine. Everything above serves that; M2b (history, search,
settings, themes, window-state) and later milestones are explicitly out.
