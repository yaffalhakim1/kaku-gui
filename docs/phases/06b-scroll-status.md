# Phase 06b — Scroll and status polish

> This phase was split from `06-abort-status.md` (2026-09-22) for pacing.
> This is **part 2 of 2**. Prerequisite: Phase 06a is complete (Esc aborts).
>
> **Status: not started.**

## What you will build

The transcript scrolls (and auto-scrolls while streaming), and the status bar
gets a state dot. Pure UI — no new HTTP, no new async.

## Concepts you will learn
- Stateful elements: why `.id(...)` must come before scroll methods.
- `ScrollHandle` and intent-based auto-scrolling.
- Returning tuples from `match`.

## Files to touch
- `src/app.rs`

---

## Step 1: Make the transcript scroll

The transcript currently grows past the window with no way to scroll. Fix
`render_messages`:

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

**`.id("messages")` is required.** `overflow_y_scroll()` and `track_scroll()`
are defined on `StatefulInteractiveElement`, not on `Styled`. Without
`.id(...)`, `div()` is a plain `Div` and neither method exists:

```
error[E0599]: no method named `overflow_y_scroll` found for struct `gpui::Div`
```

Calling `.id(...)` promotes the element to a `Stateful<Div>`, which is what
Phase 01's notes meant by "`div().id(...)` returns `Stateful<E>`".

> **Rust concept:** why `.id(...)` is needed at all
> Scrolling is stateful — GPUI has to remember the offset between frames. An element with no identity cannot be found in the next frame, so it cannot hold state. The `id` is the key GPUI uses to look the element up.

---

## Step 2: Auto-scroll to the bottom while streaming

`ScrollHandle::scroll_to_item(ix)` marks which child should be scrolled into
view, and it takes no `Window`. Add to `apply_event`, at the end of the
`TextDelta` arm:

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

`scroll_to_item` only records the intent; GPUI applies it during the next
prepaint. That is why it works from `render`'s call chain without a `Window`.

> **Rust concept:** `&self` on `scroll_to_item`
> The method takes `&self`, not `&mut self`, because the handle wraps an `Rc<RefCell<...>>` internally. Interior mutability lets it mutate shared state through a shared reference. It is the same trick as `Entity<T>`: a handle that manages its own mutability.

---

## Step 3: Status bar polish

Update the status bar to show a small dot whose color reflects the state:

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

> **Rust concept:** returning a tuple from `match`
> Both `label` and `dot_color` depend on `status`, so the match produces
> `(String, Hsla)` and destructuring assignment binds both at once. This avoids
> two separate matches.

> **Web analogy:** `.rounded_full()` ≈ `border-radius: 9999px`. `.size(px(6.0))` sets width and height together, like `width: 6px; height: 6px`.

> **Note:** this uses only colors that already exist on `Theme` (`accent`,
> `text`, `user`). Adding a `success` color is a Phase 08 concern, where the
> theme gains semantic roles.

---

## Verify

1. `cargo run`
2. Send several prompts so the transcript outgrows the window — you can
   scroll it with the mouse wheel.
3. During streaming, the view follows the newest text (auto-scroll).
4. The status bar shows a 6px dot: accent-colored when Ready, plain when
   Thinking, user-colored on Error.

## Common pitfall

`E0599: no method named 'overflow_y_scroll'` means `.id("messages")` is
missing or comes *after* the scroll methods. `.id(...)` must be called first,
because it changes which trait's methods are available on the element.

---

Once scroll and polish work, **Phase 06 is complete**. Move to
**Phase 07: Slash Commands**.
