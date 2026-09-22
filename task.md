# Kaku GUI — Task Tracker

Phases 03, 05, and 06 were split into numbered parts on 2026-09-22 for
pacing (03: 365-line single file was too much for one sitting; 05 introduced
8 concepts; 06 mixed a new action with scroll UI). The split files carry the
same content plus a corrected dependency fact: `reqwest_client` is NOT already
in Cargo.lock — adding it brings roughly 78 new crates (tokio, hyper, rustls,
h2, tower).

The roadmap and stack facts live in `AGENTS.md`. If this file disagrees with
it, `AGENTS.md` wins.

## Current state
- `src/` has: `main.rs`, `app.rs`, `theme.rs`, `input.rs`, `client/mod.rs`,
  `client/types.rs`. Compiles cleanly (0 errors, 9 warnings — all expected).
- Phases 00, 01, 02, and 03a are done.
- **Next up: Phase 03b, Step 3 only** (install HTTP client + fields already
  done and verified). Then Phase 03c (status bar).

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
- [x] Split Phase 03 into 03a/03b/03c for pacing (2026-09-22)
- [x] Phase 03a: Client crate — deps, `src/client/types.rs`, `src/client/mod.rs`,
      `mod client;`. Verified: 0 errors, 9 expected warnings.
- [x] Phase 03b Steps 1–2: `.with_http_client(Arc::new(ReqwestClient::new()))` in
      `main.rs`; `session`/`client` fields as `Option<T>` in `KakuApp`.
      Verified against user's typed code.

## Pending phases
- [ ] Phase 03b Step 3: `connect()` wired into `KakuApp::new` (the `connect`
      method is already typed and compiles; only the `app.update(cx, ...)` call
      and `render_status_bar` update remain — see 03b/03c docs)
- [ ] Phase 03c: Show session ID in status bar
- [ ] Phase 04: Send prompts
- [ ] Phase 05a: Stream plumbing — enum, channel, drain loop
- [ ] Phase 05b: Read the stream — client method, SSE reader, parsing
- [ ] Phase 05c: Streaming end-to-end — send_prompt, streaming_idx, verify
- [ ] Phase 06a: Abort — action, keybinding, client method, flag
- [ ] Phase 06b: Scroll and status polish
- [ ] Phase 07: Slash commands
- [ ] Phase 08: Waku-inspired UI/UX polish

Markdown / reasoning rendering is out of scope until Phase 07 is done.

## How to start a session
1. User says: "read phase XX"
2. Assistant reads `docs/phases/XX-*.md`.
3. Assistant explains concepts and provides code blocks.
4. User types the code and runs `cargo check` / `cargo run`.

## Notes
- Toolchain: stable Rust (MSVC target on Windows) — rustc 1.98.1 verified 2026-09-22
- Test server for later phases: `opencode serve` on `http://127.0.0.1:4096`
  (verified against OpenCode 1.18.31; live-checked `/global/health`, `/session`,
  and `/event` frame shape on 2026-09-22)
- Learning style: user does the heavy lifting; assistant guides.
- `opencode serve` warns `OPENCODE_SERVER_PASSWORD is not set; server is
  unsecured` but works without auth — no auth needed for local dev.
- GPUI is pinned to the `egoist/zed` `waku-webview` fork, same as waku. Never quote a GPUI API from memory — read the checkout in `AGENTS.md`.
- Endpoints used by Phases 03–05: `GET /global/health`, `POST /session`,
  `POST /session/{id}/prompt_async`, `POST /session/{id}/abort`, `GET /event`.
