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

## M1 — Send one request (placeholder — expand when M1 lands)

- [ ] GET `http://localhost:4400/json` renders status, duration, size, pretty JSON
- [ ] Cancel works mid-flight on `/delay/10`
- [ ] `https://nope.invalid` shows a DNS-specific message
- [ ] `/close` shows a connection error, not a crash
