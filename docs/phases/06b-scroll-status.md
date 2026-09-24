# Phase 06b — Scroll and status polish (Codex stack)

> Split from `06-abort-status.md`; part 2 of 2. Prerequisite: 06a complete
> (Esc aborts). **Revised 2026-09-23**: the scroll API notes were verified
> against the pinned checkout and two claims in the original draft were
> corrected (see the boxed note in Step 1).
>
> **Status: not started.**

## What you will build

The transcript scrolls and auto-follows the newest text while streaming,
and the status bar gets a state dot. Pure UI — no new protocol, no new async.

## Concepts you will learn
- Element identity and *why* state needs an `id`.
- `ScrollHandle`: shared, interior-mutable scroll state you own.
- Returning a tuple from `match`.

## Files to touch
- `src/app.rs`

---

## Step 1: Make the transcript scroll

Add the field and initialize it:

```rust
scroll_handle: ScrollHandle,
```

```rust
scroll_handle: ScrollHandle::new(),
```

Then `render_messages`:

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

### Where the scroll offset is stored (verified, corrected)

The pinned source shows two paths (`crates/gpui/src/elements/div.rs`, prepaint):

```rust
if let Some(scroll_handle) = self.tracked_scroll_handle.as_ref() {
    // offset comes from YOUR ScrollHandle (shared, lives in KakuApp)
} else if (overflow == Scroll) && let Some(element_state) = element_state.as_mut() {
    // offset comes from per-element state — which needs a global_id,
    // which needs .id(...)
}
```

So the precise rule is:

- `overflow_y_scroll()` **without** `track_scroll()` → GPUI stores the offset in
  per-element state, keyed by element id. **`.id("messages")` is required** —
  omit it and the wheel has nowhere to persist an offset.
- `overflow_y_scroll().track_scroll(&handle)` → the offset lives in the
  `ScrollHandle` *you* own, so identity is not what keeps it alive.

> **Correction to the original draft:** it claimed that omitting `.id(...)`
> produces `E0599: no method named overflow_y_scroll found for gpui::Div`.
> That is wrong — `impl InteractiveElement for Div` exists, so both
> `overflow_y_scroll()` and `track_scroll()` compile on a plain `div()`. The
> real consequence of a missing id is lost scroll state, not a compile error.
> Keep `.id("messages")` anyway: it makes the element stateful (stable hitbox
> and state lookups) and is required the moment you drop `track_scroll`.

> **Rust concept:** `ScrollHandle`
> `pub struct ScrollHandle(Rc<RefCell<ScrollHandleState>>)` — a newtype around
> a shared, interior-mutable cell. Interior mutability is why its methods take
> `&self` and can still mutate: the `RefCell` provides the mutability, not the
> borrow. Same idea as `Entity<T>`, in miniature.

---

## Step 2: Auto-follow while streaming

`ScrollHandle` has both a general "bring this child into view" call and an exact
"go to the bottom" call:

```rust
pub fn scroll_to_item(&self, ix: usize)              // ScrollStrategy::FirstVisible
pub fn scroll_to_top_of_item(&self, ix: usize)       // ScrollStrategy::Top
pub fn scroll_to_bottom(&self)                       // exact bottom
```

For a streaming transcript, `scroll_to_bottom()` is the right one. Add it to
the `AgentMessageDelta` arm of `apply_event`:

```rust
ServerEvent::AgentMessageDelta { delta } => {
    if let Some(idx) = self.streaming_idx {
        if let Some(message) = self.messages.get_mut(idx) {
            message.text.push_str(&delta);
            self.scroll_handle.scroll_to_bottom();
        }
    }
}
```

> **Why not `scroll_to_item(idx)`.** Its default strategy is `FirstVisible`,
> which scrolls the *minimum* needed. If the streaming message grows taller
> than the viewport, it aligns that item's **top** to the viewport top — so a
> long answer would sit with its beginning on screen and its newest words
> hidden below. `scroll_to_bottom()` sets a flag consumed in prepaint
> (`scroll_offset.y = -scroll_max.y`), which is exactly "follow the tail."

> **Rust concept:** intent, not immediate work
> `scroll_to_bottom()` only records a flag inside the handle. GPUI consumes it
> during the next prepaint. That is why this works from `apply_event` without
> a `Window` — no layout is available there, and none is needed.

---

## Step 3: Status bar polish

```rust
fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
    let thread_id = self
        .thread_id
        .clone()
        .unwrap_or_else(|| "not connected".to_string());

    let (label, dot_color) = match status {
        Status::Idle => (format!("Ready — {thread_id}"), theme.accent),
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
        .child(div().size(px(6.0)).rounded_full().bg(dot_color))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.muted)
                .child(label),
        )
}
```

> **Rust concept:** returning a tuple from `match`
> `label` and `dot_color` both depend on `status`, so one match produces
> `(String, Hsla)` and the destructuring `let` binds both. Two facts, one
> decision point — cheaper to read than two matches.

> **Web analogy:** `.rounded_full()` ≈ `border-radius: 9999px`;
> `.size(px(6.0))` sets width and height together.

Only colors that already exist on `Theme` are used (`accent`, `text`, `user`,
`muted`, `border`). A semantic `success`/`danger` pair is Phase 08 work.

---

## Verify

1. `cargo run`.
2. Send several prompts so the transcript outgrows the window: the mouse wheel
   scrolls the transcript.
3. While streaming, the view follows the newest text automatically.
4. The status bar shows a 6px dot: accent when Ready, plain when Thinking,
   purple on Error.

## Known limitation (deferred, not a bug)

Auto-follow is unconditional: scrolling up mid-stream gets yanked back to the
bottom on the next delta. Real chat clients pause following when the user
scrolls away. That needs `ScrollHandle::offset()` vs `max_offset()` and a
"stick to bottom" flag — a Phase 08 polish item, deliberately out of scope here.

---

Once scroll and polish work, **Phase 06 is complete**. Move to
**Phase 07: Slash Commands**.
