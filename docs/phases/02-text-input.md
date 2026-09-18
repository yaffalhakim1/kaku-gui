# Phase 02 — Text Input

## What you will build
A text input box where you can type, and it submits the message when you press Enter.

## Concepts you will learn
- Creating a new module (`src/input.rs`).
- `Entity<T>` — a mounted component instance.
- Handling keyboard events.
- GPUI actions and key bindings.
- `SharedString` — GPUI's optimized string type.

## Files to touch
- `src/input.rs` (new file)
- `src/app.rs`
- `src/main.rs`

---

## Step 1: Create `src/input.rs`

Create a new file `src/input.rs`:

```rust
use gpui::*;

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: SharedString::default(),
        }
    }

    pub fn content(&self) -> &SharedString {
        &self.content
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content = SharedString::default();
        cx.notify();
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .px(px(8.0))
            .rounded(px(6.0))
            .bg(rgb(0x1e1d26))
            .child(self.content.clone())
            .track_focus(&self.focus_handle)
    }
}
```

> **Rust concept:** `SharedString`
> A cheap-to-clone string used by GPUI. It is like `string` but reference-counted, so copying it does not duplicate the text in memory.

> **Rust concept:** `impl Focusable`
> This tells GPUI "this element can receive keyboard focus." Without it, keystrokes will not reach the input.

> **Rust concept:** `Default::default()` / `SharedString::default()`
> Creates an empty value. It is like `""` but typed for the specific type.

---

## Step 2: Register the module

Add this line to `src/main.rs` near the top:

```rust
mod input;
```

And add this to `src/app.rs` near the top:

```rust
use crate::input::TextInput;
```

---

## Step 3: Add `TextInput` to `KakuApp`

Add a field:

```rust
pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
    status: Status,
    input: Entity<TextInput>,
}
```

Create it in `KakuApp::new`:

```rust
cx.new(|cx| {
    let input = cx.new(|cx| TextInput::new(cx));

    Self {
        focus_handle: cx.focus_handle(),
        theme: Theme::dark(),
        messages: vec![...],
        status: Status::Idle,
        input,
    }
})
```

> **Rust concept:** `Entity<TextInput>`
> This is a handle to a living `TextInput` instance. It is like a React ref or a pointer to a mounted component.

---

## Step 4: Replace the placeholder input with the real one

In `render_input_bar`, replace the inner `div()` placeholder with:

```rust
.child(self.input.clone())
```

> **Rust concept:** `.clone()` on `Entity<T>`
> `Entity` is cheap to clone because it is reference-counted. Cloning does not duplicate the component, just the handle.

---

## Step 5: Add an action for submitting

Actions in GPUI are like custom DOM events. We will define a `SendPrompt` action and bind Enter to it.

Add this to the top of `src/main.rs`, after the `use` lines:

```rust
actions!(kaku_gui, [SendPrompt]);
```

> **Rust concept:** `actions!` macro
> This macro creates a small struct type for each action name. `SendPrompt` becomes a type you can dispatch and handle.

---

## Step 6: Bind Enter to the action

In `src/main.rs`, inside the `application().run` closure, add a key binding:

```rust
cx.bind_keys([
    KeyBinding::new("enter", SendPrompt, Some("KakuApp")),
]);
```

The `"KakuApp"` string is a key context. GPUI only fires this binding when the focused element has that context.

---

## Step 7: Handle the action in `KakuApp`

In `src/app.rs`, import `SendPrompt`:

```rust
use crate::SendPrompt;
```

In `KakuApp::new`, after creating `input`, add:

```rust
cx.on_action(
    move |_: &SendPrompt, _window: &mut Window, cx: &mut Context<Self>| {
        let content = self.input.read(cx).content().clone();
        let text = content.to_string();
        if text.trim().is_empty() {
            return;
        }

        self.messages.push(DisplayMessage {
            role: Role::User,
            text: text.clone(),
        });
        self.input.update(cx, |input, cx| input.clear(cx));
        cx.notify();
    },
);
```

Wait — this closure captures `self` mutably but is defined inside `cx.new(|cx| Self { ... })`. We cannot use `self` before `Self { ... }` is complete.

Instead, handle the action in `render` using `.on_action(cx.listener(...))` on the root div. Replace the `render` method with:

```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = self.theme;
    let messages = self.messages.clone();
    let status = self.status.clone();

    div()
        .key_context("KakuApp")
        .size_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .on_action(cx.listener(Self::send_prompt_action))
        .child(self.render_messages(messages, theme))
        .child(self.render_input_bar(theme))
        .child(self.render_status_bar(status, theme))
}
```

Then add the handler method:

```rust
impl KakuApp {
    fn send_prompt_action(
        &mut self,
        _: &SendPrompt,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let content = self.input.read(cx).content().clone();
        let text = content.to_string();
        if text.trim().is_empty() {
            return;
        }

        self.messages.push(DisplayMessage {
            role: Role::User,
            text: text.clone(),
        });
        self.input.update(cx, |input, cx| input.clear(cx));
        cx.notify();
    }
}
```

> **Rust concept:** `cx.listener(...)`
> Wraps a method on `Self` so it can be attached to an element as an event handler. It automatically gives the handler access to `self`.

> **Rust concept:** `self.input.read(cx)`
> Borrows the `TextInput` entity read-only. It is like reading from a ref.

> **Rust concept:** `self.input.update(cx, |input, cx| ...)`
> Borrows the `TextInput` entity mutably and runs a closure. It is like calling setState on a child component.

> **Rust concept:** `cx.notify()`
> Tells GPUI "this entity changed, redraw it." It is like `setState` in React.

---

## Step 8: Make `TextInput` accept keystrokes

Right now the input will not respond to typing. Add a `KeyDownEvent` handler to `TextInput::render`:

```rust
impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .px(px(8.0))
            .rounded(px(6.0))
            .bg(rgb(0x1e1d26))
            .child(self.content.clone())
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if let Some(c) = event.keystroke.key_char.as_ref() {
                    if c.len() == 1 && !event.keystroke.modifiers.shift {
                        this.content = format!("{}{}", this.content, c).into();
                        cx.notify();
                    }
                }
                if event.keystroke.key == "backspace" {
                    let mut s = this.content.to_string();
                    s.pop();
                    this.content = s.into();
                    cx.notify();
                }
            }))
    }
}
```

> **Rust concept:** `if let Some(c) = ...`
> Pattern matches on an `Option`. `Option` is like `T | undefined` in TypeScript.

> **Rust concept:** `event.keystroke.key_char.as_ref()`
> Borrows the optional char without taking ownership.

---

## Verify

1. `cargo run`
2. Click the input bar to focus it.
3. Type some text.
4. Press Enter.
5. Your text should appear as a new user message in the message list, and the input should clear.

## Common pitfall

If Enter does nothing, make sure:
- `KeyBinding::new("enter", SendPrompt, Some("KakuApp"))` is in `main.rs`.
- `.key_context("KakuApp")` is on the root div.
- `.on_action(cx.listener(Self::send_prompt_action))` is on the root div.
- The window has focus.

---

Once typing and submitting works, move to **Phase 03: Connect to OpenCode**.
