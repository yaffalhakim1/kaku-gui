# Phase 06a — Abort: Esc cancels an in-flight turn (Codex stack)

> **Revised 2026-09-22 for the Codex direction change.**
> Prerequisite: Phase 05 streams end-to-end.

## What you will build

Press Esc while the agent is working and the app sends `turn/interrupt`;
the turn finishes with `status: "interrupted"` and the UI returns to Idle.

## Concepts you will learn
- A second action (`Abort`) and its keybinding.
- Tracking the active `turn_id` so interrupt knows what to cancel.
- Reusing the `cx.spawn` + `this.update` pattern.

## Files to touch
- `src/main.rs`
- `src/app.rs`
- `src/client/mod.rs` (small addition)

---

## Step 1: Action + keybinding

In `src/main.rs`:

```rust
actions!(kaku_gui, [SendPrompt, Abort]);

// in run():
cx.bind_keys([
    KeyBinding::new("enter", SendPrompt, Some("KakuApp")),
    KeyBinding::new("escape", Abort, Some("KakuApp")),
]);
```

## Step 2: Client method

In `src/client/mod.rs` (already shown in Phase 04 — verify it's there):

```rust
pub fn interrupt(&mut self, thread_id: &str, turn_id: &str) -> Result<()> { ... }
```

## Step 3: Track the active turn

Add to `KakuApp`:

```rust
active_turn_id: Option<String>,
```

Set it when `turn/start` succeeds, clear it on `turn/completed`.

## Step 4: Handler

```rust
fn abort_action(&mut self, _: &Abort, _window: &mut Window, cx: &mut Context<Self>) {
    let (Some(client), Some(thread), Some(turn_id)) =
        (self.client.as_mut(), self.thread.as_ref(), self.active_turn_id.as_ref())
    else { return };

    let thread_id = thread.id.clone();
    let turn_id = turn_id.clone();

    // `client` needs &mut, and the task must own the call — do the send
    // synchronously here (it is a tiny write to stdin), not in a spawn.
    if let Err(e) = client.interrupt(&thread_id, &turn_id) {
        self.status = Status::Error(format!("abort: {e:#}"));
    }
    cx.notify();
}
```

Note the shape change vs the OpenCode version: `interrupt` is a cheap
*stdin write*, not a network request, so it runs inline on the UI thread.
The borrow checker is why it cannot go inside `cx.spawn` — the task would
outlive the `&mut self` borrow. (There is a way to make it work with a
channel; not needed at this scale.)

> **Rust concept:** tuple pattern in let-else
> `(Some(a), Some(b), Some(c)) = (x, y, z) else { return };` — all three must
> be present or the whole thing bails. Reads like a JS
> `if (!a || !b || !c) return;` destructure.

Register the handler on the root div:

```rust
.on_action(cx.listener(Self::abort_action))
```

## Verify

1. Send a long prompt.
2. Press Esc while "Thinking…" shows.
3. The turn completes with `status: "interrupted"` (visible as
   `turn/completed`); status returns to Ready.
4. Send another prompt — works normally.

---

Once abort works, move to **Phase 06b: scroll and status polish** (unchanged
from the OpenCode version — pure UI).
