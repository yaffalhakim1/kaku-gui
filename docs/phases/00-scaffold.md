# Phase 00 — Recap the Minimal Scaffold

## What you will learn
- What GPUI is and why it looks like "React written in Rust."
- The three files that make up the starter.
- The difference between `App`, `Entity`, `Context`, and `Render`.

## Files in this phase
- `src/main.rs`
- `src/app.rs`
- `src/theme.rs`

## Run the starter

```bash
cd C:\Users\yafit\Documents\Learn\rust\kaku-gui
cargo run
```

You should see a dark window with the text **"Hello Kaku"** in the center.

If this does not compile, run:

```bash
rustup update stable
```

> **Rust concept:** `cargo run`
> `cargo` is Rust's package manager and build tool. `cargo run` compiles your project and starts the resulting program, similar to `npm run dev`.

---

## `src/main.rs` — the entry point

```rust
mod app;
mod theme;

use gpui::*;
use gpui_platform::application;
use crate::app::KakuApp;

fn main() {
    application().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(900.0), px(640.0)),
                    cx,
                ))),
                window_min_size: Some(size(px(600.0), px(400.0))),
                ..Default::default()
            },
            |window, cx| {
                let app = KakuApp::new(window, cx);
                let focus = app.focus_handle(cx).clone();
                window.focus(&focus, cx);
                app
            },
        )
        .expect("failed to open main window");
    });
}
```

This file is like `index.tsx`. It starts the native app and opens one window.

> **Rust concept:** `mod app;`
> This tells Rust "there is a module named `app` in `src/app.rs`." It is like `import './app'` in JavaScript, but Rust needs you to declare modules explicitly.

> **Rust concept:** `use crate::app::KakuApp;`
> `crate::` means "from this project." It is like an absolute import from the project root.

> **Rust concept:** `|cx: &mut App| { ... }`
> This is a closure (anonymous function). `&mut App` means "a mutable reference to the application object." Think of `cx` as the global app context.

---

## `src/app.rs` — the root component

```rust
use gpui::*;
use crate::theme::Theme;

pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
}

impl KakuApp {
    pub fn new(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            theme: Theme::dark(),
        })
    }
}

impl Focusable for KakuApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for KakuApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(self.theme.background)
            .child(
                div()
                    .text_color(self.theme.text)
                    .text_size(px(24.0))
                    .child("Hello Kaku"),
            )
    }
}
```

This is the root UI component. It holds state and returns the element tree.

> **Web analogy:**
> - `struct KakuApp` ≈ `function KakuApp() { const [state, setState] = useState(...) }`
> - `impl Render for KakuApp` ≈ the JSX returned by the component
> - `Entity<Self>` ≈ the component instance mounted by React
> - `Context<Self>` ≈ a mix of props, state, and `setState`

> **Rust concept:** `struct`
> A `struct` is like a JavaScript object shape or TypeScript interface. It defines what data a value holds.

> **Rust concept:** `impl`
> `impl` adds methods to a type. `impl KakuApp { ... }` is like adding methods to a class.

> **Rust concept:** `&mut self`
> This means "borrow self mutably for this one method call." In React terms, it means "this method is allowed to call setState."

> **Rust concept:** `-> impl IntoElement`
> This means "this function returns something that can be turned into a GPUI element." It is like returning JSX.

---

## `src/theme.rs` — the color palette

```rust
use gpui::*;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Hsla,
    pub surface: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub user: Hsla,
    pub accent: Hsla,
    pub border: Hsla,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: rgb(0x15141b).into(),
            surface: rgb(0x1e1d26).into(),
            text: rgb(0xd5d4d6).into(),
            muted: rgb(0x6d6d6d).into(),
            user: rgb(0x8e6ad9).into(),
            accent: rgb(0xdaae76).into(),
            border: rgb(0x33323a).into(),
        }
    }
}
```

This is your CSS variables file.

> **Rust concept:** `#[derive(Clone, Copy, Debug)]`
> This auto-generates common behaviors for the struct. `Clone` = copyable, `Copy` = cheap to copy, `Debug` = printable with `{:?}`.

> **Rust concept:** `Hsla`
> GPUI's color type. The `.into()` converts `rgb(0x15141b)` (which is `Rgba`) into `Hsla` because GPUI styles expect `Hsla`.

---

## Verify

1. `cargo run` shows a dark window with "Hello Kaku."
2. Resize the window — the text stays centered.

## Common pitfall

If you see an error about `cold_path`, your Rust is older than 1.95. Run `rustup update stable`.

---

Once this runs, you are ready for **Phase 01: Static Chat Layout**.
