# Phase 04 — Codex protocol: connect and send a prompt

> **Direction change (2026-09-22):** kaku-gui now talks to **Codex App Server**
> instead of OpenCode. Phase 03's OpenCode client stays in git history as a
> completed milestone; the UI phases (00-02) carry over unchanged.
>
> Source: official Codex docs — https://developers.openai.com/codex/app-server/
> Verified against `codex-cli 0.154.0`.

## What you will build

A `CodexClient` that spawns `codex app-server` as a child process, speaks
JSON-RPC over its stdio, does the `initialize` handshake, starts a thread,
sends a prompt as a turn, and streams notifications back into the UI.

## Concepts you will learn
- Spawning a child process (`std::process::Child`, piped stdio).
- JSON-RPC 2.0 framing: requests have `method`/`params`/`id`, notifications
  omit `id`.
- Reading lines from a child's stdout on a background task.
- Mapping notifications to UI events.

## Files to touch
- `src/client/mod.rs` — rewritten (replace OpencodeClient)
- `src/client/types.rs` — rewritten (Thread, ThreadItem, notifications)
- `src/app.rs` — connect() swaps to the new handshake
- `src/main.rs` — unchanged (no HTTP client needed anymore, but leaving
  `.with_http_client` in place is harmless; remove `reqwest_client` if you
  want the dependency gone — ask first)

---

## Step 0: Understand the protocol in one screen

JSON-RPC over newline-delimited JSON on the child's stdin/stdout.

Request (has `id`):
```json
{"method":"thread/start","id":10,"params":{"cwd":"C:/your/project"}}
```

Response (echoes `id`):
```json
{"id":10,"result":{"thread":{"id":"thr_123","sessionId":"thr_123"}}}
```

Notification (no `id`, arrives whenever):
```json
{"method":"item/agentMessage/delta","params":{"threadId":"thr_123","itemId":"item_1","delta":"Hello"}}
```

Lifecycle:
1. spawn `codex app-server`
2. send `initialize` request -> read its response
3. send `initialized` notification
4. `thread/start` -> response contains `thread.id`
5. `turn/start` with `{threadId, input:[{type:"text", text}]}` -> response
   contains `turn.status: "inProgress"`
6. read notifications: `item/started`, `item/agentMessage/delta`,
   `item/completed`, `turn/completed`
7. abort: `turn/interrupt` with `{threadId, turnId}`

Everything after step 5 is notifications — the response to `turn/start` is
just an ack.

---

## Step 1: Rewrite `src/client/types.rs`

Delete the OpenCode types. New content:

```rust
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Thread {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Turn {
    pub id: String,
    pub status: String,
}

/// A parsed server notification. We only care about a few methods; unknown
/// ones become `Unknown`.
#[derive(Debug, Clone)]
pub enum ServerNotification {
    ThreadStarted,
    TurnStarted,
    TurnCompleted,
    ItemStarted,
    ItemCompleted,
    AgentMessageDelta { delta: String },
    Unknown(String),
}
```

> **Rust concept:** `#[serde(default)]` on `name`
> If the JSON omits `name`, use `None` instead of failing. Servers add fields
> over time; being liberal about what you accept keeps the client alive.

---

## Step 2: Rewrite `src/client/mod.rs`

This is the biggest new piece — a child process with two pipes. Take it in
two chunks.

**Chunk A — spawn and handshake:**

```rust
mod types;

use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use types::*;

pub struct CodexClient {
    child: Child,
}

impl CodexClient {
    pub fn spawn() -> Result<Self> {
        let mut child = Command::new("codex")
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn codex app-server (is `codex` on PATH?)")?;

        // handshake
        Self::send_raw(
            child.stdin.as_mut().expect("stdin"),
            &json!({
                "method": "initialize",
                "id": 0,
                "params": {
                    "clientInfo": {
                        "name": "kaku-gui",
                        "title": "Kaku GUI",
                        "version": "0.1.0"
                    }
                }
            }),
        )?;

        // ... read the initialize response here (Step 3) ...

        Self::send_raw(
            child.stdin.as_mut().expect("stdin"),
            &json!({ "method": "initialized", "params": {} }),
        )?;

        Ok(Self { child })
    }

    fn send_raw(stdin: &mut impl Write, value: &Value) -> Result<()> {
        let line = serde_json::to_string(value)?;
        writeln!(stdin, "{line}").context("write to codex stdin")?;
        Ok(())
    }

    /// Read one JSON line from a reader, returning None on clean EOF.
    fn read_msg(reader: &mut BufReader<impl BufRead>) -> Result<Option<Value>> {
        let mut line = String::new();
        let n = reader.read_line(&mut line).context("read from codex stdout")?;
        if n == 0 {
            return Ok(None); // EOF: process closed stdout
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Self::read_msg(reader); // skip blank lines
        }
        Ok(serde_json::from_str(trimmed)?)
    }
}
```

> **Rust concept:** `Stdio::piped()`
> Asks the OS for a pipe to the child's stdin/stdout. `Stdio::null()` throws
> stderr away. JS analogy: `spawn(cmd, args, { stdio: ['pipe', 'pipe', 'ignore'] })`.

> **Rust concept:** `as_mut().expect("stdin")`
> `child.stdin` is `Option<ChildStdin>` — it's `None` only if you didn't ask
> for a pipe. `expect` says "this can't fail, and if I'm wrong, crash loudly."
> We already asked, so it's safe.

**Chunk B — thread, turn, and the notification loop:**

```rust
impl CodexClient {
    pub fn start_thread(&mut self) -> Result<Thread> {
        Self::send_raw(
            self.child.stdin.as_mut().expect("stdin"),
            &json!({
                "method": "thread/start",
                "id": 1,
                "params": { "cwd": std::env::current_dir()? }
            }),
        )?;

        loop {
            let msg = Self::read_msg(&mut BufReader::new(
                self.child.stdout.as_mut().expect("stdout"),
            ))?
            .context("codex closed stdout during thread/start")?;

            if msg.get("id") == Some(&json!(1)) {
                if let Some(err) = msg.get("error") {
                    bail!("thread/start failed: {err}");
                }
                let thread: Thread = serde_json::from_value(
                    msg["result"]["thread"].clone(),
                )?;
                return Ok(thread);
            }
            // other messages (notifications) are ignored during handshake
        }
    }

    pub fn start_turn(&mut self, thread_id: &str, text: &str) -> Result<Turn> {
        Self::send_raw(
            self.child.stdin.as_mut().expect("stdin"),
            &json!({
                "method": "turn/start",
                "id": 2,
                "params": {
                    "threadId": thread_id,
                    "input": [{ "type": "text", "text": text }]
                }
            }),
        )?;

        loop {
            let msg = Self::read_msg(&mut BufReader::new(
                self.child.stdout.as_mut().expect("stdout"),
            ))?
            .context("codex closed stdout during turn/start")?;

            if msg.get("id") == Some(&json!(2)) {
                if let Some(err) = msg.get("error") {
                    bail!("turn/start failed: {err}");
                }
                let turn: Turn = serde_json::from_value(msg["result"]["turn"].clone())?;
                return Ok(turn);
            }
        }
    }

    pub fn interrupt(&mut self, thread_id: &str, turn_id: &str) -> Result<()> {
        Self::send_raw(
            self.child.stdin.as_mut().expect("stdin"),
            &json!({
                "method": "turn/interrupt",
                "id": 3,
                "params": { "threadId": thread_id, "turnId": turn_id }
            }),
        )?;
        Ok(())
    }

    /// Blocking read of the next notification. Returns None on clean EOF.
    /// Called from a background task (Step 3 of app.rs wiring).
    pub fn read_notification(&mut self) -> Result<Option<ServerNotification>> {
        let mut reader = BufReader::new(self.child.stdout.as_mut().expect("stdout"));
        let Some(msg) = Self::read_msg(&mut reader)? else {
            return Ok(None);
        };

        let Some(method) = msg.get("method").and_then(|m| m.as_str()) else {
            return Ok(Some(ServerNotification::Unknown(format!("{msg}"))));
        };

        let n = match method {
            "thread/started" => ServerNotification::ThreadStarted,
            "turn/started" => ServerNotification::TurnStarted,
            "turn/completed" => ServerNotification::TurnCompleted,
            "item/started" => ServerNotification::ItemStarted,
            "item/completed" => ServerNotification::ItemCompleted,
            "item/agentMessage/delta" => {
                let delta = msg["params"]["delta"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                ServerNotification::AgentMessageDelta { delta }
            }
            other => ServerNotification::Unknown(other.to_string()),
        };

        Ok(Some(n))
    }
}
```

> **Rust concept:** blocking vs async here
> Unlike the HTTP client, the child's stdout is a *blocking* file handle.
> `read_line` blocks until a line arrives. That's fine — we call
> `read_notification` from a **background thread** (via
> `cx.background_executor().spawn`), never from the UI thread. JS analogy:
> this is Node's `readline.createInterface({ input: child.stdout })` — the
> event loop handles the blocking for you; here the thread does.

> **Rust concept:** `msg.get("id") == Some(&json!(1))`
> `Value::get` returns `Option<&Value>`, and `json!(1)` builds a `Value`
> literal to compare against. Awkward-looking, but it's just "is this the
> response to request #1?"

> **Rust concept:** `loop { ... }`
> Rust's plain infinite loop. Exits via `return`/`break`. We use it to skip
> notifications until the matching response arrives.

---

## Step 3: Wire `connect` in `app.rs`

Replace the old `connect` body (no more `Url::parse`, no more `cx.http_client`):

```rust
fn connect(&mut self, cx: &mut Context<Self>) {
    cx.background_executor()
        .spawn(async move {
            // spawn + initialize + thread/start, all blocking
            let mut client = match CodexClient::spawn() {
                Ok(c) => c,
                Err(e) => { /* send Disconnected event, return */ }
            };
            // ... start_thread, then hand client + thread to the UI ...
        })
        .detach();
}
```

The full wiring (spawn -> handshake -> thread -> notify UI) goes through the
same `mpsc` channel pattern Phase 05a introduced. If you already typed 05a,
this is the payoff: `CodexClient` runs on a background task, pushes
`ServerNotification`s into the channel, and `apply_event` maps them to
message updates exactly as before.

> **Rust concept:** why the client moves into the task
> `Child` owns the pipes. Only one owner can call `read_notification` at a
> time — that's the borrow checker doing exactly the right thing: JSON-RPC
> responses and notifications share one stream, so one reader is correct.

---

## Verify

1. `codex app-server` must be reachable: `codex --version` prints a version.
2. `cargo check` — expect 0 errors (warnings from UI-only dead code are fine).
3. `cargo run`:
   - status bar leaves "not connected" (exact text is 03c-style polish)
   - a thread id (`thr_…`) appears once connected
4. Type a message — `turn/start` fires; with Phase 05a's channel in place,
   `item/agentMessage/delta` text lands in the assistant placeholder.

## Common pitfalls

- `spawn codex app-server: program not found` → `codex` not on PATH in the
  GUI process's environment.
- `Not initialized` error from any method → the `initialized` notification
  was skipped or sent before the `initialize` response arrived.
- Response never arrives → check you're reading *and* skipping notifications
  while waiting for the matching `id`.

---

Once a prompt streams text into the transcript, Phase 04+05 are effectively
one milestone on the Codex stack. Next: abort (`turn/interrupt`), scroll,
and status polish (revised 06a/06b).
