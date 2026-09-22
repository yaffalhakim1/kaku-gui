# Phase 05 — Streaming on the Codex stack

> **Revised 2026-09-22 for the Codex direction change.** The old three-part
> SSE split (05a/05b/05c) is retired — those files remain in git history.
> On the Codex stack there is no separate SSE endpoint: the JSON-RPC
> notifications on the app-server stream *are* the event stream.

## What you will build

The event plumbing (enum + channel + drain loop) and the reader task that
turns raw notifications into UI events. Phase 04's `CodexClient` already has
`read_notification()`; this phase wires it into the app.

## Parts

| Part | Scope |
|---|---|
| 05a | `UiEvent` enum, `mpsc` channel field, drain loop in `render` |
| 05b | background reader task calling `read_notification()` in a loop, pushing events |
| 05c | end-to-end: prompt streams text, `turn/completed` returns status to Idle |

Concepts carried over from the OpenCode version unchanged:
- enum variants carrying data
- `std::sync::mpsc` (many senders, one receiver)
- draining a channel inside `render` with `try_recv()`
- the `cx.spawn` / `background_executor().spawn` split

What changed vs the SSE version:
- no SSE frames or `data: ` prefix stripping — `read_msg` yields whole JSON
  values already
- `item/agentMessage/delta` **appends** (`delta` is an increment), unlike
  OpenCode's full-text `message.part.updated` — the `apply_event` arm does
  `message.text.push_str(&delta)` instead of assignment
- `turn/completed` plays the role of `session.idle`

Each part still compiles on its own with 0 errors before starting the next.
