# Phase 08 — Waku-Inspired UI/UX Polish

## What you will build
Refactor kaku-gui to look and feel like Waku: clean layout, consistent spacing, semantic surfaces, rounded composer, and polished empty states — while keeping the kaku dark color palette.

## Concepts you will learn
- Semantic color roles (surface, raised, composer, inset, border).
- Consistent spacing and typography scales.
- Centered content with `max_w`.
- Hover and active states.
- Empty states and information hierarchy.

## Files to touch
- `src/theme.rs`
- `src/app.rs`
- `src/input.rs`

---

## Step 1: Study Waku's design rules

Before typing code, look at how Waku builds its UI:

1. **Semantic colors, not raw hexes.** Waku does not scatter `rgb(0x...)` around. It uses names like `theme.surface`, `theme.raised`, `theme.composer`, `theme.border`, `theme.text_secondary`.
2. **Consistent spacing.** Most gaps and paddings are multiples of 4: `px(8.0)`, `px(12.0)`, `px(16.0)`, `px(20.0)`.
3. **Centered readable content.** The transcript and composer live inside a container with a max-width (Waku uses ~720px). This prevents text from stretching across a 4K monitor.
4. **Rounded surfaces.** The composer has a rounded rectangle background. Buttons have `rounded(px(7.0))`.
5. **Subtle borders.** Separators are 1px and low-contrast (`theme.border`).
6. **Status micro-copy.** Small, muted text at the bottom or top of the view.
7. **Empty state.** When there is no content, show a friendly centered message instead of a blank void.

> **UX principle:** "A blank screen looks broken." Always give the user a hint about what to do next.

---

## Step 2: Expand the theme with semantic roles

Open `src/theme.rs` and add more semantic color names:

```rust
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Hsla,   // window background
    pub surface: Hsla,      // main panel background
    pub raised: Hsla,       // cards / elevated surfaces
    pub composer: Hsla,     // input bar background
    pub inset: Hsla,        // nested dark wells
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    pub user: Hsla,
    pub assistant: Hsla,
    pub accent: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: rgb(0x15141b).into(),
            surface: rgb(0x1b1a22).into(),
            raised: rgb(0x23222b).into(),
            composer: rgb(0x212029).into(),
            inset: rgb(0x13121a).into(),
            text: rgb(0xd5d4d6).into(),
            text_secondary: rgb(0xa3a3a3).into(),
            text_muted: rgb(0x6d6d6d).into(),
            user: rgb(0x8e6ad9).into(),
            assistant: rgb(0xd5d4d6).into(),
            accent: rgb(0xdaae76).into(),
            border: rgb(0x33323a).into(),
            border_strong: rgb(0x4a4950).into(),
            success: rgb(0x58d8ad).into(),
            warning: rgb(0xdaae76).into(),
            danger: rgb(0xd85d5d).into(),
        }
    }
}
```

> **Rust concept:** semantic naming
> Naming colors by role instead of value makes the UI easier to change later. `composer` tells you where it is used; `rgb(0x212029)` does not.

---

## Step 3: Add a header bar

In `src/app.rs`, add a `render_header` helper:

```rust
fn render_header(&self, theme: Theme) -> impl IntoElement {
    div()
        .h(px(44.0))
        .px(px(16.0))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text)
                .child("Kaku"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.text_muted)
                .child("Ctrl+Q to quit"),
        )
}
```

> **Web analogy:** `FontWeight::SEMIBOLD` ≈ `font-weight: 600`.

---

## Step 4: Render the empty state

Add an empty-state helper:

```rust
fn render_empty_state(&self, theme: Theme) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(12.0))
        .child(
            div()
                .text_size(px(32.0))
                .text_color(theme.accent)
                .child("Kaku"),
        )
        .child(
            div()
                .text_size(px(14.0))
                .text_color(theme.text_secondary)
                .child("Ask anything to start a conversation"),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme.text_muted)
                .child("Press Enter to send, Esc to abort"),
        )
}
```

> **UX principle:** The empty state tells the user three things: brand, purpose, and how to begin.

---

## Step 5: Refactor messages to a centered transcript

Update `render_messages` to look like Waku's transcript pane:

```rust
fn render_messages(&self, messages: Vec<DisplayMessage>, theme: Theme) -> impl IntoElement {
    div()
        .id("messages")
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
                .mx_auto()
                .p(px(20.0))
                .flex()
                .flex_col()
                .gap(px(20.0))
                .children(messages.into_iter().map(move |m| self.render_message(m, theme))),
        )
}
```

> **Web analogy:** `.max_w(px(720.0)).mx_auto()` ≈ `max-width: 720px; margin: 0 auto;`.

Update `render_message`:

```rust
fn render_message(&self, m: DisplayMessage, theme: Theme) -> impl IntoElement {
    let (label, color) = match m.role {
        Role::User => ("You", theme.user),
        Role::Assistant => ("Assistant", theme.assistant),
        Role::System => ("System", theme.text_muted),
    };

    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(color)
                .child(label),
        )
        .child(
            div()
                .text_size(px(14.0))
                .leading_relaxed()
                .text_color(theme.text)
                .child(m.text),
        )
}
```

> **UX principle:** Labeling each message with "You" / "Assistant" improves scannability. Users do not have to guess who said what.

---

## Step 6: Refactor the composer to look like Waku

Update `render_input_bar` to a centered, rounded composer:

```rust
fn render_input_bar(&self, _status: Status, theme: Theme) -> impl IntoElement {
    div()
        .px(px(20.0))
        .pb(px(16.0))
        .pt(px(8.0))
        .bg(theme.surface)
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
                .mx_auto()
                .p(px(12.0))
                .rounded(px(12.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.composer)
                .shadow_sm()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(self.input.clone()),
        )
}
```

> **Web analogy:** `.shadow_sm()` ≈ a small CSS box-shadow. It lifts the composer off the page.

---

## Step 7: Style `TextInput` to match

Update `src/input.rs` so the input blends into the composer:

```rust
impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .h(px(24.0))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(0xd5d4d6))
            .child(self.content.clone())
            .track_focus(&self.focus_handle)
    }
}
```

The composer now provides the border and background, so the input itself is just text.

---

## Step 8: Update the root `render`

Replace the root `render` method with:

```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    if let Some(rx) = self.events.take() {
        while let Ok(ev) = rx.try_recv() {
            self.apply_event(ev, cx);
        }
        self.events = Some(rx);
    }

    let theme = self.theme;
    let messages = self.messages.clone();
    let status = self.status.clone();
    let empty = messages.is_empty();

    div()
        .key_context("KakuApp")
        .size_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .on_action(cx.listener(Self::send_prompt_action))
        .on_action(cx.listener(Self::abort_action))
        .child(self.render_header(theme))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .bg(theme.surface)
                .child(if empty {
                    self.render_empty_state(theme).into_any_element()
                } else {
                    self.render_messages(messages, theme).into_any_element()
                })
                .child(self.render_input_bar(status.clone(), theme)),
        )
        .child(self.render_status_bar(status, theme))
}
```

> **Rust concept:** `.into_any_element()`
> Converts different element types into a common `AnyElement` so they can appear in the same branch.

---

## Step 9: Polish the status bar

Update `render_status_bar` to be minimal and left-aligned:

```rust
fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
    let label = match status {
        Status::Idle => "Ready",
        Status::Busy => "Thinking…",
        Status::Error(ref e) => e.as_ref(),
    };

    div()
        .h(px(22.0))
        .px(px(16.0))
        .flex()
        .flex_row()
        .items_center()
        .bg(theme.background)
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .text_size(px(10.0))
                .text_color(theme.text_muted)
                .child(label.to_string()),
        )
}
```

---

## Verify

1. `cargo run`
2. You should see:
   - A header bar with "Kaku" on the left.
   - A centered empty state: "Ask anything to start a conversation."
   - A rounded composer at the bottom.
   - A thin status bar.
3. Send a message.
4. Messages should appear in a centered column with "You" / "Assistant" labels.

## Common pitfall

If the layout looks stretched, make sure `.max_w(px(720.0)).mx_auto()` is applied to both the transcript inner container and the composer inner container.

---

Congratulations — you now have a Waku-polished native chat client.

From here you can keep exploring:
- Add a left sidebar for multiple sessions.
- Render markdown/code blocks in assistant messages.
- Add subtle animations for message appearance.
- Persist window size and theme preference.
