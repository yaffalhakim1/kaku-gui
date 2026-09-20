# Kaku GUI — Task Tracker

The roadmap and stack facts live in `AGENTS.md`. If this file disagrees with it, `AGENTS.md` wins.

## Current state
- `src/` has four files: `main.rs`, `app.rs`, `theme.rs`, `input.rs`. Compiles cleanly.
- Phases 00, 01, and 02 are done. **Next up: Phase 03.**
- No `src/client/` yet — it is typed by hand in Phase 03.
- Phase 03 carries an approved dependency change: `reqwest_client`, `serde`,
  `serde_json`, and `futures` join the approved set. `reqwest` and `tokio` are
  **not** added; see `AGENTS.md`.
- Ready for the Phase 03 tutorial session.

## Completed
- [x] Strip scaffold to `main.rs`, `app.rs`, `theme.rs`
- [x] Create `AGENTS.md` with workflow context
- [x] Create phase markdowns 00–08
- [x] Verify scaffold compiles with `cargo check`
- [x] Correct `AGENTS.md` with verified stack facts (fork pin, GPUI API rules, tutorial-mode rules)
- [x] Phase 00: Recap scaffold
- [x] Phase 01: Static chat layout
- [x] Phase 02: Text input — `src/input.rs` with `TextInput`, `SendPrompt` action bound to Enter
- [x] Re-verify `AGENTS.md` against the pinned checkout at rev `57bd4fe`; drop the
      nonexistent `kaku-tui` donor; fix the dependency rule and the HTTP guidance
- [x] Re-ground Phases 03–07 on the GPUI HTTP stack (`reqwest_client` +
      `gpui::http_client`), replacing `reqwest`/`tokio`
- [x] Fix the `cx.spawn` closure shape in Phases 04 and 06 (`|this, mut cx|`
      does not compile; the `async move |this, cx|` form does)
- [x] Fix Phase 05's SSE parsing: `message.part.updated` has no `delta` field,
      so the UI assigns the part text instead of appending it
- [x] Fix Phase 06's scroll step (`.id(...)` is required before
      `overflow_y_scroll`/`track_scroll`) and its use of a nonexistent
      `theme.success`
- [x] Verify the full Phase 03–06 code compiles together against the pinned fork

## Pending phases (for future sessions)
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
  (verified against OpenCode 1.18.31)
- Learning style: user does the heavy lifting; assistant guides.
- GPUI is pinned to the `egoist/zed` `waku-webview` fork, same as waku. Never quote a GPUI API from memory — read the checkout in `AGENTS.md`.
- Endpoints used by Phases 03–05: `GET /global/health`, `POST /session`,
  `POST /session/{id}/prompt_async`, `POST /session/{id}/abort`, `GET /event`.
