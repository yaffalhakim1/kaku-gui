# Phase 05 — Send prompts and stream the reply (Codex stack)

> **Revised 2026-09-22 (Codex direction change).** The old SSE split
> (05a/05b/05c) is retired — on the Codex stack the JSON-RPC notifications
> *are* the stream, and the channel + drain loop already landed in 04b.

## What you will build

`send_prompt_action` sends `turn/start` through the `CodexClient` handle,
pushes a user message and an empty assistant placeholder, and
`apply_event` streams `AgentMessageDelta` into it.

## Concepts
- Fire-and-forget requests: `turn/start` acks instantly; the reply arrives
  as notifications.
- `AgentMessageDelta` **appends** (`push_str`) — each delta is an increment,
  unlike OpenCode's full-text events.
- `TurnStarted` gives you the `turn_id` to remember (Phase 06a needs it).
- `TurnCompleted` returns the status to Idle and clears the placeholder
  index.

## Shape

```rust
// send_prompt_action, after the guards:
let Some(client) = self.client.clone() else { ... };
let Some(thread_id) = self.thread_id.clone() else { ... };

client.request("turn/start", json!({
    "threadId": thread_id,
    "input": [{ "type": "text", "text": text }],
}))?;
```

`apply_event` gains the `TurnStarted` / `AgentMessageDelta` /
`TurnCompleted` arms, plus `streaming_idx: Option<usize>` routing — the
same placeholder pattern from the OpenCode-era Phase 04.

## Verify

1. `cargo run` — type a prompt, press Enter.
2. The assistant placeholder fills as deltas arrive (append!).
3. Status returns to `Ready` when the turn completes.

## Common pitfall

Text appends twice → you assigned AND appended, or the drain loop runs the
same event twice. The drain loop must consume; `try_recv` does.
