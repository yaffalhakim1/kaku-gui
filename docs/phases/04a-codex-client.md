# Phase 04a — CodexClient: spawn, handshake, request handle

> This phase was split from `04-send-prompts.md` (2026-09-22) for pacing and
> correctness. Prerequisite: Phase 03c compiles with 0 errors.
>
> **Status: not started.**

## What you will build

`src/client/` becomes a Codex App Server client: spawn `codex app-server`
as a child process, do the JSON-RPC `initialize` handshake over its stdio,
and expose a cloneable handle the UI can use to send requests.

## Concepts you will learn
- Child processes: `Command`, `Stdio::piped()`, taking the pipes.
- JSON-RPC framing: requests have `method`/`params`/`id`; notifications omit `id`.
- `BufReader` + `read_line` and the `Ok(0)` EOF rule.
- Why the stdin handle and stdout reader need *separate* owners.

## Files to touch
- `src/client/types.rs` — rewritten
- `src/client/mod.rs` — rewritten

---

## Step 0: The protocol in one screen

```json
{"method":"thread/start","id":10,"params":{}}        <- request (has id)
{"id":10,"result":{"thread":{"id":"thr_123"}}}       <- response (echoes id)
{"method":"thread/started","params":{"thread":{..}}} <- notification (no id, whenever)
```

Lifecycle: spawn -> `initialize` -> read its response -> `initialized`
notification -> then `thread/start` / `turn/start` / `turn/interrupt`, and
read notifications forever.

Verified against the official docs (developers.openai.com/codex/app-server)
and `codex-cli 0.154.0`.

---

## Step 1: Rewrite `src/client/types.rs`

```rust
/// A Codex thread — the conversation. Replaces OpenCode's `Session`.
#[derive(Deserialize, Debug, Clone)]
pub struct Thread {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Events the reader forwards to the UI. ServerNotification becomes this
/// after classification, so the UI never sees raw JSON.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Carries the client handle to the UI exactly once, after the
    /// handshake succeeded.
    Ready { client: crate::client::CodexClient },
    ThreadStarted { id: String },
    TurnStarted { turn_id: String },
    AgentMessageDelta { delta: String },
    ItemCompleted,
    TurnCompleted,
    Disconnected(String),
    /// Responses (which have an `id`, not a `method`) and unknown
    /// notifications land here; the UI ignores them.
    Ignored,
}
```

> **Rust concept:** `#[serde(default)]`
> If the JSON omits `name`, use `None` instead of failing. Servers add
> fields over time; accept liberally.

> **Rust concept:** why `Ready` carries the client
> The client handle is *created* on a background task (04b) but *used* by
> the UI. A channel event is how it crosses over. `CodexClient` is
> `Clone`, so the event can carry it.

---

## Step 2: Rewrite `src/client/mod.rs`

```rust
mod types;

use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub use types::*;

/// Handle for SENDING requests to codex app-server. Cloned freely.
/// The stdout reader (04b) is separate on purpose: it blocks on
/// `read_line`, and whatever holds it must not block the UI.
#[derive(Clone)]
pub struct CodexClient {
    stdin: Arc<Mutex<ChildStdin>>,
    next_id: Arc<AtomicU64>,
}

impl CodexClient {
    /// Spawn `codex app-server`, run the initialize handshake, and return
    /// the request handle plus the stdout reader.
    pub fn spawn() -> Result<(Self, BufReader<ChildStdout>)> {
        let mut child = Command::new("codex")
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn codex app-server (is `codex` on PATH?)")?;

        let mut stdin = child.stdin.take().expect("stdin piped");
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        drop(child); // the OS keeps the process running; we keep the pipes

        // -- initialize: the server rejects everything before this --
        write_line(
            &mut stdin,
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

        // read messages until the response with id 0 arrives
        loop {
            let msg = read_msg(&mut stdout)?.context("codex closed stdout during initialize")?;
            if msg.get("id") == Some(&json!(0)) {
                if let Some(err) = msg.get("error") {
                    bail!("initialize failed: {err}");
                }
                break;
            }
            // anything else is a stray notification; ignore it
        }

        // -- initialized: acknowledges the handshake --
        write_line(&mut stdin, &json!({ "method": "initialized", "params": {} }))?;

        Ok((
            Self {
                stdin: Arc::new(Mutex::new(stdin)),
                next_id: Arc::new(AtomicU64::new(1)),
            },
            stdout,
        ))
    }

    /// Fire-and-forget request. Returns the id used. The response arrives
    /// on the stdout reader later; 04b's loop forwards or ignores it.
    pub fn request(&self, method: &str, params: Value) -> Result<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = json!({ "method": method, "id": id, "params": params });
        let mut stdin = self.stdin.lock().expect("stdin poisoned");
        write_line(&mut stdin, &msg)?;
        Ok(id)
    }
}

fn write_line(stdin: &mut ChildStdin, value: &Value) -> Result<()> {
    let line = serde_json::to_string(value)?;
    writeln!(stdin, "{line}").context("write to codex stdin")?;
    Ok(())
}

/// Read one JSON message. `Ok(None)` = clean EOF (the child exited).
pub fn read_msg(reader: &mut BufReader<ChildStdout>) -> Result<Option<Value>> {
    let mut line = String::new();
    let n = reader.read_line(&mut line).context("read from codex stdout")?;
    if n == 0 {
        return Ok(None);
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return read_msg(reader); // skip blank lines
    }
    Ok(serde_json::from_str(trimmed).context("bad JSON from codex"))
}
```

> **Rust concept:** `child.stdin.take()`
> `Child` stores its pipes as `Option`. `take()` moves the pipe out, leaving
> `None` behind. You can't move a field out of a struct you only own a
> reference to, and `take` is the standard "extract and leave a hole" move.

> **Rust concept:** `Arc<Mutex<ChildStdin>>`
> Two wrappers, each with one job. `Arc` = shared ownership (many clones,
> one value). `Mutex` = only one writer at a time. The UI (and later the
> abort handler) clones the handle and writes requests; stdout is *not*
> shared — it moves to the reader thread. JS analogy: one
> `child.stdin` you share by reference, one `readline` consumer on stdout.

> **Rust concept:** why responses are ignored, not awaited
> `thread/start` and `turn/start` both have matching *notifications*
> (`thread/started`, `turn/started`) that carry the same ids. The reader
> forwards those; the UI never waits on a response. This sidesteps
> request/response correlation entirely.

> **Rust concept:** `AtomicU64` + `fetch_add`
> A counter that is safe to bump from multiple clones of the handle at
> once. `fetch_add` returns the old value and stores old+1 — your
> thread-safe `id++`.

---

## Verify

```bash
cargo check
```

Expect **0 errors**. Warnings about `ServerEvent` variants never constructed
and `CodexClient`/`read_msg` never used are expected — 04b wires them.

`cargo run` behaves exactly as at the end of Phase 03c (old client still in
`app.rs`); nothing visible changed yet.

---

Once this compiles clean, move to **Phase 04b: connect wiring and the
reader thread**.
