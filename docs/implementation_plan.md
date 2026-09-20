> **HISTORICAL NOTES — not authoritative.**
> The roadmap and stack facts live in `AGENTS.md`. Where this file disagrees
> with it, `AGENTS.md` wins. The "Phase 0" below describes how the scaffold was
> _created_ in an earlier session; in the tutorial, Phase 00 is a recap of the
> scaffold that already exists. Phases 8–9 here are older, more ambitious
> sketches than the current `docs/phases/` files.

# Kaku GUI — Implementation Plan

A native desktop client for OpenCode, built with Rust + GPUI (the same stack as [waku](https://github.com/egoist/waku)). This plan assumes a single-crate app that talks directly to a running `opencode serve` instance, talking to the OpenCode HTTP API directly.

## Architecture decisions

- **Single crate.** No daemon split. The app holds `OpencodeClient` directly.
- **Entity-driven UI.** One root `Entity<KakuApp>` implements GPUI's `Render` trait.
- **Async bridge.** SSE runs on GPUI's own executor (`smol`-based), not tokio: `cx.background_executor().spawn(...)` returning a `Task<T>`. Results flow into the UI through a `std::sync::mpsc::channel` and are drained inside `render`. HTTP goes through GPUI's built-in `HttpClient` (`cx.http_client()`), so no `reqwest`.
- **No markdown in v0.** Plain text only. Markdown/reasoning blocks come later.

## Phase 0 — Project scaffold ✅

**Goal:** A compiling GPUI window with the kaku dark theme.

- [x] Create `Cargo.toml` with `gpui` from the `egoist/zed` `waku-webview` fork.
- [x] Create `src/main.rs` that opens a centered window.
- [x] Create `src/app.rs` with `KakuApp` entity and `Render` impl.
- [x] Create `src/theme.rs` with kaku colors.
- [x] `src/input.rs` is created by hand in Phase 02.
- [ ] `src/client/` is created by hand in Phase 03.

**Verification:** `cargo check` passes and a dark window opens.

## Phase 1 — Static chat layout

**Goal:** Render a hardcoded message list, input bar, and status bar.

- Add a few static `DisplayMessage` entries in `KakuApp::new`.
- Render them in a scrollable column with role prefixes (`› ` for user).
- Render the input bar with the tan prompt glyph and the `TextInput` entity.
- Render the status bar with `Idle` / `Busy` / `Error` states.
- Add `gap`, `padding`, `border_t_1`, and `rounded` styling.

**Verification:** Run the app and see the static layout.

## Phase 2 — Custom TextInput

**Goal:** Type into the input bar and submit on Enter.

- Handle `KeyDownEvent` in `TextInput`.
- Append printable characters to `content`.
- Support Backspace to delete the last grapheme.
- Emit a `SendPrompt` action on Enter.
- Clear the input after submit.
- Add a blinking caret (optional but nice).

**Verification:** Keystrokes appear in the input bar; Enter clears it.

## Phase 3 — Connect to OpenCode

**Goal:** Create a session on startup and display model/session info.

- Read CLI args / env vars in `main.rs`:
  - `OPENCODE_SERVER_URL` (default `http://127.0.0.1:4096`)
  - `KAKU_GUI_PASSWORD` or `OPENCODE_SERVER_PASSWORD`
  - `OPENCODE_SERVER_USERNAME` (default `opencode`)
- In `KakuApp::new`, spawn a GPUI background task (`cx.background_executor()`, not tokio) to:
  - Build `OpencodeClient`.
  - Call `health()`.
  - Call `create_session(Some("kaku-gui"))`.
  - Call `default_model()`.
- Send the result back through a channel and store `session`, `default_model`, and `client` on `KakuApp`.
- Show the model name and session title in the status bar.

**Verification:** With `opencode serve` running, the app starts and shows the connected model.

## Phase 4 — Send prompts

**Goal:** Submit a user message and pre-create the assistant placeholder.

- Implement `KakuApp::send_prompt`.
- On `SendPrompt` action:
  - Read `input.content()`.
  - Append a `Role::User` message to `messages`.
  - Append an empty `Role::Assistant` message.
  - Set `streaming_idx` to the assistant index.
  - Set `status` to `Busy`.
  - Call `client.send_prompt(session_id, text, None).await` on a background task.
- Clear the input.
- Handle send errors by setting `Status::Error`.

**Verification:** Typed text appears as a user message; an empty assistant message appears below it.

## Phase 5 — SSE streaming

**Goal:** Stream assistant responses from `GET /event` into the UI.

- Run the SSE reader on `cx.background_executor()`, feeding the UI through a channel that `render` drains.
- Use `std::sync::mpsc::channel` (or `crossbeam-channel`) to send `StreamEvent` into the UI.
- Start the SSE reader after `create_session` succeeds.
- In `KakuApp::render`, drain the receiver and call `apply_event` for each event.
- The `apply_event` shape:
  - `PartUpdated.delta` present → append to assistant message.
  - `PartUpdated.text` only → replace assistant message.
  - `SessionIdle` → set `Idle`, clear `streaming_idx`.
  - `Disconnected` / `SessionError` → set `Error` if busy.
- Filter out user-text echoes using `last_user_text`.

**Verification:** Ask a question; the assistant response streams in character by character.

## Phase 6 — Abort, scroll, and status polish

**Goal:** Make the chat usable during long turns.

- Implement `Abort` action: call `client.abort(session_id)` and set `abort_requested`.
- Auto-scroll the message list to the bottom when `messages` change.
- Show connection state and model name in the status bar.
- Add a subtle loading indicator while `Busy`.
- Clamp status-bar error messages to one line.

**Verification:** Press Esc during streaming; the assistant stops and status returns to `Idle`.

## Phase 7 — Commands and local state

**Goal:** Add `/` commands and persistence.

- Add a small `src/commands.rs` for local slash commands.
- Support `/clear`, `/model`, `/quit`, `/help`.
- Persist window size/position to a JSON file in `~/.config/kaku-gui/`.
- Persist last used model override.

**Verification:** `/clear` empties the transcript; window bounds restore on relaunch.

## Phase 8 — Markdown and reasoning (advanced)

**Goal:** Rich assistant message rendering.

- Integrate a markdown renderer (study Waku's `src/md/` or use `pulldown-cmark` + custom GPUI elements).
- Render code blocks with a monospaced font and background wash.
- Render reasoning blocks in a collapsible muted panel.
- Render tool-call events as compact activity pills.

**Verification:** Responses with code fences render with monospace styling.

## Phase 9 — Multi-session and sidebar (advanced)

**Goal:** Manage multiple chat sessions.

- Add a left sidebar listing past sessions.
- Store session metadata in SQLite or JSON.
- Allow creating, renaming, and deleting sessions.
- Persist messages per session.

**Verification:** Switch between two sessions without losing history.

## Reference files

| File                  | Purpose                                                                   |
| --------------------- | ------------------------------------------------------------------------- |
| `src/main.rs`         | App bootstrap, window open, global key bindings                           |
| `src/app.rs`          | Root entity, render tree, state mutation                                  |
| `src/input.rs`        | Text input entity (Phase 02 — planned, not yet created)                   |
| `src/theme.rs`        | Color palette                                                             |
| `src/client/mod.rs`   | HTTP client and SSE request builder (Phase 03 — planned, not yet created) |
| `src/client/types.rs` | OpenCode wire types (Phase 03 — planned, not yet created)                 |

## Risks and mitigations

1. **GPUI API churn.** Pin the same git revision as waku and read Zed source when docs are missing.
2. **Text input complexity.** Build the minimal input first; only add caret/selection after send/receive works.
3. **Long transcripts.** Virtualize the message list with GPUI's `list()` once Phase 5 is stable.

## Current phase

Scaffold is complete. Tutorial sessions start at Phase 00 (recap); the roadmap lives in `AGENTS.md`.
