# Kaku GUI — Task Tracker

> **Direction change (2026-09-22):** kaku-gui now targets **Codex App Server**
> (JSON-RPC over stdio) instead of OpenCode (HTTP REST + SSE). Phases 00-03
> are complete and untouched by the change. Phase 04 onward has been revised;
> the old OpenCode versions of 04/05a-c/06a-b/07 remain in git history
> (commit `9065b0c`).
>
> Source of truth for the Codex protocol: official docs at
> https://developers.openai.com/codex/app-server/ — verified against
> `codex-cli 0.154.0`.

## Current state
- `src/` has: `main.rs`, `app.rs`, `theme.rs`, `input.rs`, `client/mod.rs`,
  `client/types.rs` (OpenCode version, to be replaced in Phase 04).
- Phases 00-03 complete, verified 0 errors, status bar shows a session id.
- `src/client/` still speaks OpenCode; Phase 04 rewrites it as `CodexClient`.

## Completed
- [x] Scaffold, AGENTS.md, phase docs 00-08
- [x] Phase 00: recap scaffold
- [x] Phase 01: static chat layout
- [x] Phase 02: text input, Enter to submit
- [x] Phase 03a: client types (OpenCode stack — now historical)
- [x] Phase 03b: HTTP client install + startup connect
- [x] Phase 03c: session id in status bar
- [x] Commit + push `9065b0c` (Phase 03 complete on the OpenCode stack)
- [x] Decision: switch to Codex App Server; revise Phase 04+ docs

## Pending phases (Codex stack)
- [ ] Phase 04: `CodexClient` — spawn `codex app-server`, stdio JSON-RPC,
      initialize handshake, `thread/start`, `turn/start`
- [ ] Phase 05a: `UiEvent` enum + mpsc channel + render drain loop
- [ ] Phase 05b: background reader task over `read_notification()`
- [ ] Phase 05c: end-to-end streaming (`item/agentMessage/delta` appends,
      `turn/completed` -> Idle)
- [ ] Phase 06a: Abort via `turn/interrupt`, `active_turn_id` tracking
- [ ] Phase 06b: scroll + status polish (unchanged from old version)
- [ ] Phase 07: slash commands — `/clear`, `/new` (thread/start), `/model`
      (model/list), `/quit`
- [ ] Phase 08: Waku-inspired UI/UX polish (unchanged)

Markdown / reasoning rendering remains out of scope until Phase 07 is done.

## How to start a session
1. User says: "read phase XX"
2. Assistant reads `docs/phases/XX-*.md`.
3. Assistant explains concepts and provides code blocks.
4. User types the code and runs `cargo check` / `cargo run`.

## Notes
- Toolchain: stable Rust (MSVC) — rustc 1.98.1 verified 2026-09-22.
- Codex CLI: `codex-cli 0.154.0` verified on this machine.
- The OpenCode-era `reqwest_client` dependency becomes unnecessary once
  Phase 04 lands; removing it is a small cleanup, ask first (approved-set rule).
- GPUI fork pin and API rules live in `AGENTS.md` and are unchanged.
