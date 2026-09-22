# Phase 04b — Connect on startup: reader thread and event channel

> Prerequisite: Phase 04a compiles with 0 errors.
>
> **Status: not started.**

## What you will build

Swap `app.rs` from the OpenCode client to `CodexClient`: `connect` spawns
the process on a background task, hands stdout to a blocking reader loop,
and pushes `ServerEvent`s through an mpsc channel that `render` drains.
By the end, the status bar shows a real thread id (`thr_…`).

## Concepts you will learn
- `std::sync::mpsc` — many senders, one receiver.
- Draining a channel inside `render` without fighting the borrow checker.
- Why the reader loop must never touch the UI directly.

## Files to touch
- `src/app.rs`
- `src/main.rs` (one line: remove the HTTP client install — optional)

---

## Step 1: Swap the fields on `KakuApp`

Remove `session`/`client` (OpenCode era). Add:

```rust
pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
    status: Status,
    input: Entity<TextInput>,
    thread_id: Option<String>,
    client: Option<CodexClient>,
    events: Option<std::sync::mpsc::Receiver<ServerEvent>>,
}
```

Initialize all three new fields as `None` in `new`.

Imports at the top of `src/app.rs` — replace the OpenCode ones:

```rust
use crate::client::{read_msg, CodexClient, ServerEvent};
use serde_json::json;
```

`main.rs` may keep `.with_http_client(...)` for now; it is harmless. When
you want it gone, delete `reqwest_client` from `Cargo.toml` and the two
lines in `main.rs` (the approved-set rule: this one is *removing*, so it is
allowed — ask anyway if unsure).

---

## Step 2: Replace `connect`

```rust
fn connect(&mut self, cx: &mut Context<Self>) {
    let (tx, rx) = std::sync::mpsc::channel::<ServerEvent>();
    self.events = Some(rx);

    cx.background_executor()
        .spawn(async move {
            let (client, mut stdout) = match CodexClient::spawn() {
                Ok(pair) => pair,
                Err(e) => {
                    let _ = tx.send(ServerEvent::Disconnected(format!("connect: {e:#}")));
                    return;
                }
            };

            let _ = tx.send(ServerEvent::Ready { client });

            if let Err(e) = std::env::current_dir() {
                let _ = tx.send(ServerEvent::Disconnected(format!("cwd: {e:#}")));
                return;
            }
            // note: client was moved into the Ready event above, so the
            // thread/start request is sent by the UI in 05 — for 04b the
            // reader loop below is already the interesting part.
        })
        .detach();
}
```

Wait — `client` was moved into `Ready`. To send `thread/start` we need it
before handing it over. Reorder: clone the handle first.

Corrected body:

```rust
fn connect(&mut self, cx: &mut Context<Self>) {
    let (tx, rx) = std::sync::mpsc::channel::<ServerEvent>();
    self.events = Some(rx);

    cx.background_executor()
        .spawn(async move {
            let (client, mut stdout) = match CodexClient::spawn() {
                Ok(pair) => pair,
                Err(e) => {
                    let _ = tx.send(ServerEvent::Disconnected(format!("connect: {e:#}")));
                    return;
                }
            };

            let cwd = match std::env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ServerEvent::Disconnected(format!("cwd: {e:#}")));
                    return;
                }
            };

            let send = client.request("thread/start", json!({ "cwd": cwd }));
            if let Err(e) = send {
                let _ = tx.send(ServerEvent::Disconnected(format!("thread/start: {e:#}")));
                return;
            }

            let _ = tx.send(ServerEvent::Ready { client });

            loop {
                match read_msg(&mut stdout) {
                    Ok(Some(msg)) => {
                        if tx.send(classify(msg)).is_err() {
                            return; // receiver dropped: app closing
                        }
                    }
                    Ok(None) => {
                        let _ = tx.send(ServerEvent::Disconnected("codex exited".to_string()));
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(ServerEvent::Disconnected(format!("{e:#}")));
                        return;
                    }
                }
            }
        })
        .detach();
}
```

> **Rust concept:** `cx.background_executor().spawn(async move { ... })`
> Unlike `cx.spawn` (which gives you `this`/the entity), the background
> executor takes a plain async block with no UI access. Everything the
> block needs must be moved in — here, the `tx` sender. It talks to the UI
> only through the channel. That is the whole safety story: a blocking
> `read_line` can never freeze the UI because it runs on a pool thread.

> **Rust concept:** `if tx.send(...).is_err() { return; }`
> `send` fails only when the receiver is gone — meaning the app is closing.
> Returning ends the reader loop and lets the child process handle be
> dropped.

---

## Step 3: `classify` — raw JSON to `ServerEvent`

Add at the bottom of `src/app.rs`:

```rust
fn classify(msg: serde_json::Value) -> ServerEvent {
    let Some(method) = msg.get("method").and_then(|m| m.as_str()) else {
        return ServerEvent::Ignored; // a response (has id, no method)
    };
    let params = &msg["params"];

    match method {
        "thread/started" => ServerEvent::ThreadStarted {
            id: params["thread"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        },
        "turn/started" => ServerEvent::TurnStarted {
            turn_id: params["turn"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        },
        "item/agentMessage/delta" => ServerEvent::AgentMessageDelta {
            delta: params["delta"].as_str().unwrap_or_default().to_string(),
        },
        "item/completed" => ServerEvent::ItemCompleted,
        "turn/completed" => ServerEvent::TurnCompleted,
        _ => ServerEvent::Ignored,
    }
}
```

> **Rust concept:** `params["thread"]["id"]`
> `Value` implements `Index`, so `msg["params"]["thread"]["id"]` walks the
> JSON like `obj?.params?.thread?.id` in JS — except a wrong path yields
> `Value::Null` instead of throwing. `.as_str()` then converts
> `Null`/string to `Option<&str>`.

---

## Step 4: Drain events in `render`

At the top of `render`, before building the UI:

```rust
while let Some(event) = self.events.as_ref().and_then(|rx| rx.try_recv().ok()) {
    self.apply_event(event, cx);
}
```

And `apply_event`:

```rust
impl KakuApp {
    fn apply_event(&mut self, event: ServerEvent, cx: &mut Context<Self>) {
        match event {
            ServerEvent::Ready { client } => self.client = Some(client),
            ServerEvent::ThreadStarted { id } => {
                self.thread_id = Some(id);
                self.status = Status::Idle;
            }
            ServerEvent::Disconnected(why) => {
                self.status = Status::Error(format!("disconnected: {why}"));
            }
            _ => {}
        }
        cx.notify();
    }
}
```

> **Rust concept:** the drain loop
> `self.events.as_ref()` borrows the receiver; `try_recv()` never blocks;
> `.ok()` turns the Err(empty/closed) into `None`, ending the `while let`.
> The borrow ends each iteration, so `apply_event` can freely mutate `self`.
> JS analogy: `while ((e = queue.poll())) handle(e)`.

---

## Step 5: Status bar reads the thread id

In `render_status_bar`, swap the session lookup for:

```rust
let thread_id = self
    .thread_id
    .clone()
    .unwrap_or_else(|| "not connected".to_string());
```

and use it in the `Idle` arm: `format!("Ready — {thread_id}")`.

---

## Verify

1. `codex --version` works.
2. `cargo check` — 0 errors (dead-code warnings for `TurnStarted`,
   `AgentMessageDelta`, `TurnCompleted`, `ItemCompleted` are expected —
   Phase 05 uses them).
3. `cargo run`:
   - status bar goes from `Ready — not connected` to `Ready — thr_…` within
     a second.
   - closing the app exits cleanly.

## Common pitfalls

- `spawn codex app-server: program not found` → `codex` not on the GUI
  process's PATH.
- Status stays `not connected` with no error → check the terminal: the
  `Disconnected` event sets an error in the status bar; no error and no
  thread id usually means `thread/started` never matched `classify`.
- `bad JSON from codex` → you may have pointed `Command::new` at something
  that is not `codex app-server`.

---

Once `thr_…` shows in the status bar, Phase 04 is complete. Move to
**Phase 05: send prompts and stream the reply**.
