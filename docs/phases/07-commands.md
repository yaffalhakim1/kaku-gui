# Phase 07 — Slash Commands

## What you will build
Support local-only slash commands: `/clear`, `/model`, and `/quit`.

## Concepts you will learn
- Creating a new module (`src/commands.rs`).
- String slicing and parsing in Rust.
- `std::process::exit`.
- Conditional logic based on message content.

## Files to touch
- `src/commands.rs` (new file)
- `src/app.rs`
- `src/main.rs`

---

## Step 1: Create `src/commands.rs`

```rust
use crate::app::{KakuApp, Status};
use gpui::Context;

#[derive(Clone, Debug)]
pub enum Command {
    Clear,
    Model(String),
    Quit,
    Unknown(String),
}

pub fn parse(input: &str) -> Command {
    let trimmed = input.trim();
    if trimmed == "/clear" {
        return Command::Clear;
    }
    if trimmed == "/quit" {
        return Command::Quit;
    }
    if let Some(rest) = trimmed.strip_prefix("/model ") {
        return Command::Model(rest.trim().to_string());
    }
    Command::Unknown(trimmed.to_string())
}

impl KakuApp {
    pub fn execute_command(
        &mut self,
        cmd: Command,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        match cmd {
            Command::Clear => {
                self.messages.clear();
                cx.notify();
            }
            Command::Model(model) => {
                self.status = Status::Error(format!("model override set to: {model}"));
                cx.notify();
            }
            Command::Quit => {
                std::process::exit(0);
            }
            Command::Unknown(text) => {
                self.messages.push(crate::app::DisplayMessage {
                    role: crate::app::Role::System,
                    text: format!("Unknown command: {text}"),
                });
                cx.notify();
            }
        }
    }
}
```

> **Rust concept:** `strip_prefix`
> Removes a prefix from a string if it exists, returning the rest. Like `str.startsWith(...)` combined with `slice`.

> **Rust concept:** `self.messages.clear()`
> Empties the vector in place.

---

## Step 2: Register the module

Add to `src/main.rs`:

```rust
mod commands;
```

---

## Step 3: Route commands in `send_prompt_action`

In `src/app.rs`, update `send_prompt_action` to detect commands before sending:

```rust
fn send_prompt_action(
    &mut self,
    _: &SendPrompt,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    let content = self.input.read(cx).content().clone();
    let text = content.to_string();
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }

    self.input.update(cx, |input, cx| input.clear(cx));

    if text.starts_with('/') {
        self.messages.push(DisplayMessage {
            role: Role::System,
            text: text.clone(),
        });
        let cmd = crate::commands::parse(&text);
        self.execute_command(cmd, window, cx);
        return;
    }

    // ... rest of the existing send logic
}
```

> **Rust concept:** `text.starts_with('/')`
> Checks if a string begins with a character. This is how we distinguish commands from normal prompts.

---

## Verify

1. `cargo run`
2. Type `/clear` and press Enter — the chat should empty.
3. Type `/model anthropic/claude-opus-4` — status bar shows the override.
4. Type `/quit` — the app closes.
5. Type `/unknown` — a system message says "Unknown command."

## Common pitfall

If commands are sent to OpenCode instead of running locally, make sure the `text.starts_with('/')` check happens before the HTTP call.

---

Congratulations! You have completed the core kaku-gui tutorial.

From here you can explore:
- Phase 08 (advanced): render markdown, code blocks, and reasoning.
- Persist sessions to disk.
- Add a sidebar for multiple sessions.
