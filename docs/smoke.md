# Manual smoke checklist

Run on **macOS and Windows** at every milestone exit. Start the fixture server
first: `bun run fixtures` (http://localhost:4400).

## M0 — Foundation

- [ ] `bun install && bun run build` succeeds from a clean checkout
- [ ] `bun run test` and `cargo test` (in `src-tauri/`) pass
- [ ] `bun tauri dev` opens a window titled "request-kit" with no white flash
      (window appears only after content is ready)
- [ ] Window size/position persist across relaunch
- [ ] Launching a second instance focuses the existing window instead of
      opening a new one
- [ ] `~/.request-kit/` is created on first run (mode `0700` on macOS)
- [ ] CI builds installers on macos-latest and windows-latest
- [ ] Unsigned installers launch manually on both target systems

## M1 — Send one request

Fixture base: `http://localhost:4400` (`bun run fixtures`).

- [ ] GET `/json` → 2xx green badge, duration, decoded size, pretty-printed JSON in
      the Pretty view; Raw view shows the verbatim body; Headers view lists the
      normalized headers
- [ ] GET `/delay/10`, then **Cancel** mid-flight → request stops, response panel
      shows a muted "Request cancelled." (not an error), no lingering spinner
- [ ] GET `/redirect/3` → follows the chain (redirects toggle on) to a 2xx; final
      URL in the meta bar reflects the last hop
- [ ] Toggle **Follow redirects → Off** in Settings, resend `/redirect/3` → a 3xx
      response is shown (not followed)
- [ ] GET `/gzip` → body is decoded and rendered (no raw gzip bytes); size is the
      decoded size
- [ ] GET `/dup-headers` → the duplicated header name appears twice in Headers view
- [ ] GET `/size/5000` → "truncated" flag in the meta bar + **Save to file…**
      writes the full body via the Rust save dialog; cancelling the dialog is silent
- [ ] GET `/binary` → binary notice + **Save to file…** (no garbled text render)
- [ ] GET `/close` → connection error headline, app does not crash
- [ ] GET `https://nope.invalid` → DNS-specific message ("Could not resolve the
      host…"); collapsed Details carries the redacted chain
- [ ] Malformed URL (e.g. `ht!tp://x`) → validation error shown, no send attempted
- [ ] POST `/echo` with a JSON body and no Content-Type → `application/json` sent;
      a syntactically bad JSON body shows the lint position on **Format**
- [ ] **Stale-completion guard:** start `/delay/10`, then immediately send `/json`
      in the same tab → only the `/json` result renders; the slow `/delay/10`
      completion neither clobbers the display nor lingers in retention
- [ ] `mod+Enter` sends the active request; the Send button flips to Cancel while
      in flight
- [ ] Copy body copies the displayed text to the clipboard

## M2a — Storage and saved requests

Exercises `~/.request-kit` persistence. No fixture server needed.

- [ ] Sidebar **+** creates a collection; it appears and enters inline rename
- [ ] Right-click a collection → **New request** → the request opens in the tab
- [ ] Edit the request (method/URL/body), press `mod+S` → dirty dot clears
- [ ] Right-click → **New folder**; create a request inside it (nesting works)
- [ ] Quit and relaunch → the full tree, names, and saved request contents are
      restored (create → save → quit → relaunch survives)
- [ ] Right-click a request → **Rename** (inline) → name updates and persists
- [ ] Right-click a request → **Duplicate** → a "… copy" appears; original intact
- [ ] Right-click → **Delete** a request that is open → confirm → the tab becomes
      an unsaved scratch tab (edits retained), node gone after relaunch
- [ ] Delete a folder with children → confirm → whole subtree removed
- [ ] On macOS: `~/.request-kit` is `0700`, `collection.json`/request files `0600`
      (`ls -le`); files are human-readable JSON with a UUID `id`
- [ ] Hand-drop a corrupt JSON file and a second file with a duplicate `id` into a
      collection dir → relaunch → app still starts, both are quarantined
      (`.corrupt-<ts>`), and the sidebar shows the "files could not be loaded"
      notice; the duplicate winner is the lexicographically-first path

## M2b — History, search, settings, themes, window-state

Fixture server (`bun run fixtures`) helps for the history/redaction checks.

- [ ] Send `GET /json?token=secret` → the **History** tab lists the request as
      `?token=<redacted>` (template only); no `Authorization`/secret text appears
- [ ] History shows method + status (or error kind) + relative time, newest first
- [ ] Click a history row for a saved request → it reopens that request; click a
      row for an unsaved request → a scratch tab is prefilled from the template URL
- [ ] **Clear** (confirm) empties the history list and `history/history.jsonl`
- [ ] Open **Settings** (gear or `mod+,`) → change theme to Dark → UI updates
      immediately; relaunch → theme persists
- [ ] Change theme to System → matches the OS appearance and follows OS changes
- [ ] Change **Default timeout** in Settings → a new request without a per-request
      timeout uses it (e.g. set 500 ms, hit `/delay/2` → times out)
- [ ] Change **Font size** → editors/UI reflow; persists across relaunch
- [ ] Search box filters the collections tree by name (containers on the path
      are kept; a matching collection shows its whole subtree)
- [ ] Move the window / resize, quit, relaunch → geometry restored; drag mostly
      off-screen, relaunch → clamped back on-screen
- [ ] Hand-corrupt `settings.json` → relaunch → app starts with default settings
      and the file is quarantined (`.corrupt-<ts>`)
