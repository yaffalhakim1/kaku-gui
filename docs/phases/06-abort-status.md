# Phase 06 — Abort, Scroll, and Status Polish

## What you will build
Press Esc to cancel an in-flight prompt, keep the message list scrolled to the bottom, and polish the status bar.

## Concepts you will learn
- Handling a second action (`Abort`).
- Scrolling a GPUI container.
- Mutable state flags (`abort_requested`).

## Files to touch
- `src/main.rs`
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

---

## Step 2: Add `abort` to the client

In `src/client/mod.rs`, add:

```rust
pub async fn abort(&self, session_id: &str) -> Result<()> {
    let url = self.base.join(&format!("/session/{session_id}/abort"))?;
    self.http
        .post(url)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
```

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

    cx.spawn(|_this, mut cx| async move {
        let result = client.abort(&session_id).await;
        cx.update(|cx| {
            _this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.status = Status::Error(format!("abort: {e:#}"));
                    cx.notify();
                }
            })
            .ok();
        })
        .ok();
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

---

## Step 5: Auto-scroll the message list

Replace `render_messages` with a version that keeps the list scrolled. For this phase, we will simply reverse the display so new messages appear at the bottom naturally, or use a `ScrollHandle`.

Simpler approach: add an `id` to the messages container and a `ScrollHandle`. For now, add a `ScrollHandle` field:

```rust
pub struct KakuApp {
    ...
    scroll_handle: ScrollHandle,
}
```

Initialize:

```rust
scroll_handle: ScrollHandle::new(),
```

Update `render_messages`:

```rust
fn render_messages(&self, messages: Vec<DisplayMessage>, theme: Theme) -> impl IntoElement {
    div()
        .id("messages")
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(16.0))
        .overflow_y_scroll()
        .track_scroll(&self.scroll_handle)
        .children(messages.into_iter().map(move |m| self.render_message(m, theme)))
}
```

In `apply_event`, after updating messages, scroll to bottom:

```rust
self.scroll_handle.scroll_to_end(cx);
```

Wait — `apply_event` takes `&mut Context<Self>`, and `scroll_to_end` likely needs `&mut Window`. Since `apply_event` is called from `render`, we don't have `Window` there.

For this phase, skip auto-scroll and just make sure new messages render. Add a TODO comment:

```rust
// TODO: auto-scroll requires passing Window into apply_event or using a subscription.
```

Instead, we can use `contain_scroll` or just let the layout push content down.

Actually, the simplest working scroll is to use `overflow_y_scroll()` on the messages container. New content will appear and the user can scroll manually. Auto-scroll can be a later polish.

So update `render_messages` to:

```rust
fn render_messages(&self, messages: Vec<DisplayMessage>, theme: Theme) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(16.0))
        .overflow_y_scroll()
        .children(messages.into_iter().map(move |m| self.render_message(m, theme)))
}
```

If `overflow_y_scroll()` is not available in your GPUI version, use `.overflow_hidden()` and revisit later.

---

## Step 6: Status bar polish

Update the status bar to show a small dot when busy:

```rust
fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
    let session_id = self
        .session
        .as_ref()
        .map(|s| s.id.clone())
        .unwrap_or_else(|| "not connected".to_string());

    let (label, dot_color) = match status {
        Status::Idle => (format!("Ready — {session_id}"), theme.success),
        Status::Busy => ("Thinking…".to_string(), theme.accent),
        Status::Error(ref e) => (format!("Error: {e}"), theme.user),
    };

    div()
        .h(px(24.0))
        .px(px(12.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .size(px(6.0))
                .rounded_full()
                .bg(dot_color),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.muted)
                .child(label),
        )
}
```

> **Web analogy:** `.rounded_full()` ≈ `border-radius: 9999px`.

---

## Verify

1. `cargo run`
2. Send a long prompt.
3. While "Thinking…" is showing, press Esc.
4. The stream should stop and the status bar should return to "Ready."

## Common pitfall

If Esc does nothing, make sure:
- `Abort` is declared in the actions macro.
- The key binding uses `"escape"`.
- `.on_action(cx.listener(Self::abort_action))` is on the root div.

---

Once abort and status polish work, move to **Phase 07: Slash Commands**.
