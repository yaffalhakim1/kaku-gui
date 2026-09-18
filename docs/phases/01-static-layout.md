# Phase 01 — Static Chat Layout

## What you will build
Render a hardcoded list of chat messages, plus an input bar and a status bar.

## Concepts you will learn
- `Vec<T>` — Rust's dynamic array.
- `.clone()` — making a copy so Rust's borrow checker is happy.
- Helper methods that return GPUI elements.
- Flexbox layout with GPUI.

## Files to touch
- `src/app.rs`
- `src/theme.rs` (no changes needed, but you will use it)

---

## Step 1: Add message types

In `src/app.rs`, after the `use` lines, add two enums and one struct:

```rust
#[derive(Clone, Debug)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug)]
pub struct DisplayMessage {
    pub role: Role,
    pub text: String,
}
```

> **Rust concept:** `enum`
> An `enum` is a type that can be one of several variants. Here a message is either `User`, `Assistant`, or `System`. It is like a union type in TypeScript.

> **Rust concept:** `pub`
> `pub` makes something visible outside its module. Without `pub`, it is private like a non-exported function.

---

## Step 2: Add hardcoded messages to `KakuApp`

Add a `messages` field to `KakuApp` and initialize it with two fake messages:

```rust
pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
}
```

In `KakuApp::new`, initialize it:

```rust
cx.new(|cx| Self {
    focus_handle: cx.focus_handle(),
    theme: Theme::dark(),
    messages: vec![
        DisplayMessage {
            role: Role::User,
            text: "Hello, who are you?".to_string(),
        },
        DisplayMessage {
            role: Role::Assistant,
            text: "I am an AI assistant running inside a native GPUI app.".to_string(),
        },
    ],
})
```

> **Rust concept:** `Vec<T>`
> A growable list, like JavaScript's `Array<T>`.

> **Rust concept:** `.to_string()`
> Converts a string literal (`&str`) into an owned `String`.

---

## Step 3: Add a `Status` enum

Add this near the top of `src/app.rs`:

```rust
#[derive(Clone, Debug)]
pub enum Status {
    Idle,
    Busy,
    Error(String),
}
```

Then add a `status` field to `KakuApp` and initialize it:

```rust
pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
    status: Status,
}
```

```rust
cx.new(|cx| Self {
    focus_handle: cx.focus_handle(),
    theme: Theme::dark(),
    messages: vec![...],
    status: Status::Idle,
})
```

---

## Step 4: Create helper methods

Add a new `impl KakuApp` block at the bottom of `src/app.rs`:

```rust
impl KakuApp {
    fn render_messages(&self, messages: Vec<DisplayMessage>, theme: Theme) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(16.0))
            .children(messages.into_iter().map(move |m| self.render_message(m, theme)))
    }

    fn render_message(&self, m: DisplayMessage, theme: Theme) -> impl IntoElement {
        let (prefix, color) = match m.role {
            Role::User => ("› ", theme.user),
            Role::Assistant => ("", theme.text),
            Role::System => ("", theme.muted),
        };

        div()
            .flex()
            .flex_row()
            .gap(px(4.0))
            .child(div().text_color(color).child(prefix.to_string()))
            .child(div().text_color(theme.text).child(m.text))
    }

    fn render_input_bar(&self, theme: Theme) -> impl IntoElement {
        div()
            .h(px(48.0))
            .px(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .border_t_1()
            .border_color(theme.border)
            .child(div().text_color(theme.accent).child("› "))
            .child(
                div()
                    .flex_1()
                    .h(px(32.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .rounded(px(6.0))
                    .bg(theme.surface)
                    .child("Type a message..."),
            )
    }

    fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
        let label = match status {
            Status::Idle => "Ready",
            Status::Busy => "Thinking…",
            Status::Error(ref e) => e.as_ref(),
        };

        div()
            .h(px(24.0))
            .px(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted)
                    .child(label.to_string()),
            )
    }
}
```

> **Rust concept:** `&self`
> This means "read-only borrow." The method can look at `self` but cannot change it. It is like a React component reading props/state without calling setState.

> **Rust concept:** `move` closure
> `messages.into_iter().map(move |m| ...)` tells the closure to take ownership of captured variables. For now, just know it is often needed when returning elements from helper methods.

> **Web analogy:**
> - `div()` ≈ `<div>`
> - `.flex()` ≈ `display: flex`
> - `.flex_col()` ≈ `flex-direction: column`
> - `.gap(px(8.0))` ≈ `gap: 8px`
> - `.p(px(16.0))` ≈ `padding: 16px`

---

## Step 5: Use the helpers in `render`

Replace the body of `render` with this:

```rust
fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    let theme = self.theme;
    let messages = self.messages.clone();
    let status = self.status.clone();

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .child(self.render_messages(messages, theme))
        .child(self.render_input_bar(theme))
        .child(self.render_status_bar(status, theme))
}
```

> **Rust concept:** `let messages = self.messages.clone();`
> We clone the messages so we can pass them to `render_messages` without holding a borrow of `self`. Rust only allows one mutable reference at a time, so cloning avoids conflicts.

---

## Verify

Run:

```bash
cargo run
```

You should see:
- A dark window.
- The user message "› Hello, who are you?" near the top.
- The assistant message below it.
- A bottom input bar with the prompt glyph `› `.
- A status bar that says "Ready."

## Common pitfall

If you get a borrow checker error about `self`, make sure you cloned `messages` and `status` into local variables before passing them to helper methods.

---

Once this compiles and looks right, move to **Phase 02: Text Input**.
