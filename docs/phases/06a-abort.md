# Phase 06a — Abort: Esc cancels an in-flight prompt

> This phase was split from `06-abort-status.md` (2026-09-22) for pacing.
> This is **part 1 of 2**. Prerequisite: Phase 05 is complete end-to-end.
>
> **Status: not started.**

## What you will build

Press Esc while the assistant is responding and the request is cancelled:
the app calls the server's abort endpoint, and the status returns to "Ready."
when the stream ends.

## Concepts you will learn
- A second action (`Abort`) and its keybinding.
- A mutable state flag (`abort_requested`) and why it must be cleared on completion.
- Reusing the `cx.spawn` + `this.update` pattern from Phase 03b.

## Files to touch
- `src/main.rs`
- `src/client/mod.rs`
- `src/app.rs`

---

## Step 1: Add the `Abort` action

In `src/main.rs`, update the actions macro:

```rust
actions!(kaku_gui, [SendPrompt, Abort]);
```

Add a key binding:

```rust
cx.bind_keys([
    KeyBinding::new("enter", SendPrompt, Some("KakuApp")),
    KeyBinding::new("escape", Abort, Some("KakuApp")),
]);
```

> **Note:** `Some("KakuApp")` (no double parens) is the warning-free spelling.
> The existing `Some(("KakuApp"))` on the enter binding also compiles — the
> inner parens are redundant and produce a warning only.

---

## Step 2: Add `abort` to the client

In `src/client/mod.rs`, add:

```rust
pub async fn abort(&self, session_id: &str) -> Result<()> {
    let url = self.url(&format!("/session/{session_id}/abort"))?;

    let response = self
        .http
        .post_json(url.as_str(), AsyncBody::empty())
        .await
        .context("POST abort")?;

    if !response.status().is_success() {
        anyhow::bail!("POST abort -> {}", response.status());
    }

    Ok(())
}
```

`AsyncBody::empty()` is the empty request body. `post_json` sets
`Content-Type: application/json` even when the body is empty, which this
endpoint accepts.

---

## Step 3: Add `abort_requested` flag

Add to `KakuApp`:

```rust
pub struct KakuApp {
    ...
    abort_requested: bool,
}
```

Initialize:

```rust
abort_requested: false,
```

---

## Step 4: Handle the Abort action

Import `Abort` in `src/app.rs`:

```rust
use crate::{Abort, SendPrompt};
```

Add the action handler to the root div in `render`:

```rust
.on_action(cx.listener(Self::send_prompt_action))
.on_action(cx.listener(Self::abort_action))
```

Add the method:

```rust
fn abort_action(&mut self, _: &Abort, _window: &mut Window, cx: &mut Context<Self>) {
    let Some(session) = self.session.as_ref() else { return };
    if !matches!(self.status, Status::Busy) {
        return;
    }

    let Some(client) = self.client.clone() else { return };
    let session_id = session.id.clone();

    self.abort_requested = true;
    cx.notify();

    cx.spawn(async move |this, cx| {
        if let Err(e) = client.abort(&session_id).await {
            let _ = this.update(cx, |this, cx| {
                this.status = Status::Error(format!("abort: {e:#}"));
                cx.notify();
            });
        }
    })
    .detach();
}
```

Then update `apply_event` for `StreamEvent::Idle`:

```rust
StreamEvent::Idle => {
    if self.abort_requested {
        self.abort_requested = false;
        self.status = Status::Idle;
    } else {
        self.status = Status::Idle;
    }
    self.streaming_idx = None;
}
```

> **Rust concept:** `matches!(self.status, Status::Busy)`
> Returns `true` if `status` is the `Busy` variant. It is a concise way to check enum variants.

> **Rust concept:** why the flag exists
> The abort HTTP call and the stream ending are asynchronous and independent.
> The flag records "the user asked to stop" so that when `session.idle`
> eventually arrives, the app knows the return to Idle was user-initiated. The
> flag must be reset when consumed, or one Esc press would poison every future
> turn.

> **Rust concept:** the `let-else` guard chain
> `let Some(session) = ... else { return };` then `if !matches!(...) { return }`
> then `let Some(client) = ... else { return };` — three early-exit guards, each
> naming exactly why it bails. Compare with nested `if let` + `if` + `if let`:
> same logic, flat shape.

---

## Verify

1. `cargo run`
2. Send a long prompt (e.g. "write a long story").
3. While "Thinking…" is showing, press Esc.
4. The stream should stop and the status bar should return to "Ready."
5. Send another prompt — it should work normally (the flag was reset).

## Common pitfall

If Esc does nothing, make sure:
- `Abort` is declared in the actions macro.
- The key binding uses `"escape"`.
- `.on_action(cx.listener(Self::abort_action))` is on the root div.

---

Once abort works, move to **Phase 06b: Scroll and status polish**.
