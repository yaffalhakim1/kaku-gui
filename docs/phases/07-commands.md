# Phase 07 — Slash Commands (Codex stack)

> **Revised 2026-09-22 for the Codex direction change.**

## What you will build
Local-only slash commands: `/clear`, `/model`, `/quit` — plus `/new` which
starts a fresh Codex thread (`thread/start`), replacing OpenCode-era
session thinking.

## Concepts you will learn
- A new module (`src/commands.rs`).
- String parsing (`strip_prefix`).
- `model/list` over JSON-RPC.

## Changes vs the OpenCode version

- `/new` is *new*: sends `thread/start`, stores the returned `thread.id`,
  clears the transcript. This is the natural "new conversation" on Codex.
- `/model <name>`: sends `model/list`, matches the user's text against
  `displayName`/`id`, then passes `model` in the next `turn/start` params.
  (The old "local echo only" caveat goes away — Codex takes a `model`
  override per turn.)
- `/clear`, `/quit`, `/unknown` behave exactly as before (local only).

## Files to touch
- `src/commands.rs` (new)
- `src/app.rs`
- `src/main.rs` (`mod commands;`)
- `src/client/mod.rs` (`list_models()` + `start_turn` gaining an optional
  model param)

## `list_models` sketch

```rust
pub fn list_models(&mut self) -> Result<Vec<ModelInfo>> {
    // id: 4, method: "model/list", params: { "limit": 20 }
    // read responses until id 4 arrives; parse result.data
}
```

`ModelInfo { id, display_name, is_default }` — same `Deserialize` pattern as
`Thread`.

## Verify

1. `/clear` empties the chat.
2. `/model gpt-5.6` matches a model from `model/list` and reports it.
3. `/new` starts a fresh thread id (`thr_…`) and clears the transcript.
4. `/quit` exits.
5. `/bogus` shows "Unknown command."

---

Once commands work, move to **Phase 08: Waku-inspired UI/UX polish**
(unchanged — pure UI).
