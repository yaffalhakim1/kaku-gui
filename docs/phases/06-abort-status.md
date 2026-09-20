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

`AsyncBody::empty()` is the empty request body. `post_json` sets `Content-Type: application/json` even when the body is empty, which this endpoint accepts.

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

---

## Step 5: Make the transcript scroll

The transcript currently grows past the window with no way to scroll. Fix `render_messages`:

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
        .children(
            messages
                .into_iter()
                .map(move |m| self.render_message(m, theme)),
        )
}
```

Add the field and initialize it:

```rust
scroll_handle: ScrollHandle,
```

```rust
scroll_handle: ScrollHandle::new(),
```

**`.id("messages")` is required.** `overflow_y_scroll()` and `track_scroll()` are defined on `StatefulInteractiveElement`, not on `Styled`. Without `.id(...)`, `div()` is a plain `Div` and neither method exists:

```
error[E0599]: no method named `overflow_y_scroll` found for struct `gpui::Div`
```

Calling `.id(...)` promotes the element to a `Stateful<Div>`, which is what Phase 01's notes meant by "`div().id(...)` returns `Stateful<E>`".

> **Rust concept:** why `.id(...)` is needed at all
> Scrolling is stateful — GPUI has to remember the offset between frames. An element with no identity cannot be found in the next frame, so it cannot hold state. The `id` is the key GPUI uses to look the element up.

### Auto-scrolling to the bottom

`ScrollHandle::scroll_to_item(ix)` marks which child should be scrolled into view, and it takes no `Window`. Add to `apply_event`, at the end of the `TextDelta` arm:

```rust
StreamEvent::TextDelta { text } => {
    if let Some(idx) = self.streaming_idx {
        if let Some(message) = self.messages.get_mut(idx) {
            message.text = text;
            self.scroll_handle.scroll_to_item(idx);
        }
    }
}
```

`scroll_to_item` only records the intent; GPUI applies it during the next prepaint. That is why it works from `render`'s call chain without a `Window`.

> **Rust concept:** `&self` on `scroll_to_item`
> The method takes `&self`, not `&mut self`, because the handle wraps an `Rc<RefCell<...>>` internally. Interior mutability lets it mutate shared state through a shared reference. It is the same trick as `Entity<T>`: a handle that manages its own mutability.
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
        Status::Idle => (format!("Ready — {session_id}"), theme.accent),
        Status::Busy => ("Thinking…".to_string(), theme.text),
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

> **Web analogy:** `.rounded_full()` ≈ `border-radius: 9999px`. `.size(px(6.0))` sets width and height together, like `width: 6px; height: 6px`.

> **Note:** this uses only colors that already exist on `Theme` (`accent`, `text`, `user`). Adding a `success` color is a Phase 08 concern, where the theme gains semantic roles.

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
