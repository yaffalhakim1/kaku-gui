# Kaku GUI — Task Tracker

The roadmap and stack facts live in `AGENTS.md`. If this file disagrees with it, `AGENTS.md` wins.

## Current state
- Scaffold is 3 files (`src/main.rs`, `src/app.rs`, `src/theme.rs`) and compiles cleanly.
- Phase markdowns 00–08 exist under `docs/phases/` (9 files, ending at `08-waku-uiux.md`).
- No `src/input.rs` and no `src/client/` yet — those are typed by hand in Phases 02 and 03.
- Ready for tutorial sessions.

## Completed
- [x] Strip scaffold to `main.rs`, `app.rs`, `theme.rs`
- [x] Create `AGENTS.md` with workflow context
- [x] Create phase markdowns 00–08
- [x] Verify scaffold compiles with `cargo check`
- [x] Correct `AGENTS.md` with verified stack facts (fork pin, GPUI API rules, tutorial-mode rules)

## Pending phases (for future sessions)
- [ ] Phase 00: Recap scaffold
- [ ] Phase 01: Static chat layout
- [ ] Phase 02: Text input
- [ ] Phase 03: Connect to OpenCode
- [ ] Phase 04: Send prompts
- [ ] Phase 05: SSE streaming
- [ ] Phase 06: Abort, scroll, status polish
- [ ] Phase 07: Slash commands
- [ ] Phase 08: Waku-inspired UI/UX polish

Markdown / reasoning rendering is out of scope until Phase 07 is done.

## How to start a session
1. User says: "read phase XX"
2. Assistant reads `docs/phases/XX-*.md`.
3. Assistant explains concepts and provides code blocks.
4. User types the code and runs `cargo check` / `cargo run`.

## Notes
- Toolchain: stable Rust (MSVC target on Windows)
- Test server for later phases: `opencode serve` on `http://127.0.0.1:4096`
- Learning style: user does the heavy lifting; assistant guides.
- GPUI is pinned to the `egoist/zed` `waku-webview` fork, same as waku. Never quote a GPUI API from memory — read the checkout in `AGENTS.md`.
