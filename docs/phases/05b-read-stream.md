# Phase 05b — Read the stream: SSE reader task

> This phase was split from `05-sse-streaming.md` (2026-09-22) for pacing.
> This is **part 2 of 3**. Prerequisite: Phase 05a compiles with 0 errors.
>
> **Status: not started.**

## What you will build

The client method that opens the SSE connection and the background task that
reads it line by line, parses each frame, and pushes events into the channel
from 05a. The UI is not touched beyond starting the reader — wiring it into
`connect` happens here, applying text to messages happened in 05a.

## Concepts you will learn
- Server-Sent Events (SSE) frame format.
- Running a blocking reader on the background executor.
- `BufReader` + `read_line` and the `Ok(0)` end-of-stream rule.
- `serde_json::Value` and `pointer()` for shapeless JSON.

## Files to touch
- `src/client/mod.rs`
- `src/app.rs`

---

## Step 1: Add an event-stream method to the client

In `src/client/mod.rs`, add `Response` to the existing `gpui::http_client` import:

```rust
use gpui::http_client::{AsyncBody, HttpClient, Json, Response, Url};
```

And the method inside `impl OpencodeClient`:

```rust
pub async fn event_stream(&self) -> Result<Response<AsyncBody>> {
    let url = self.url("/event")?;
    let response = self
        .http
        .get(url.as_str(), AsyncBody::empty(), true)
        .await
        .context("GET /event")?;

    if !response.status().is_success() {
        anyhow::bail!("GET /event -> {}", response.status());
    }

    Ok(response)
}
```

Note what this returns: the **unread response**, not a parsed body. SSE is an
endless stream, so you cannot `read_to_string` it — that would wait forever.
The caller reads it line by line instead.

That is also why the client needs no new imports beyond `Response`: the
streaming happens in `app.rs`, and the client only hands over the response.

> **Rust concept:** why the import list changed
> `Response` and `AsyncBody` are the same types Phase 03 already used, but `Response` was not needed by name before. A function that *returns* a type must name it in its signature.

---

## Step 2: Start the SSE reader after connecting

In `connect`, in the `Ok((client, session))` branch, start the reader:

```rust
Ok((client, session)) => {
    this.client = Some(client);
    this.session = Some(session);
    this.status = Status::Idle;
    this.start_sse_reader(cx);
}
```

Then add these imports at the top of `src/app.rs` — the reader runs here, not
in the client:

```rust
use futures::{AsyncBufReadExt, io::BufReader};
```

And add the method:

```rust
impl KakuApp {
    fn start_sse_reader(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else {
            return;
        };

        let (tx, rx) = std::sync::mpsc::channel();
        self.events = Some(rx);

        cx.background_executor()
            .spawn(async move {
                let response = match client.event_stream().await {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(StreamEvent::Disconnected(format!("sse: {e:#}")));
                        return;
                    }
                };

                let mut reader = BufReader::new(response.into_body());
                let mut line = String::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) => {
                            let _ = tx.send(StreamEvent::Disconnected(format!("sse: {e:#}")));
                            return;
                        }
                    }

                    let Some(json) = line.trim_end().strip_prefix("data: ") else {
                        continue;
                    };

                    let Ok(wrapper) = serde_json::from_str::<SseWrapper>(json) else {
                        continue;
                    };

                    if tx.send(classify(wrapper)).is_err() {
                        return;
                    }
                }

                let _ = tx.send(StreamEvent::Disconnected("closed".to_string()));
            })
            .detach();
    }
}
```

Add the helper type and functions at the bottom of `src/app.rs`:

```rust
#[derive(Debug, serde::Deserialize)]
struct SseWrapper {
    #[serde(rename = "type")]
    type_: String,
    #[serde(default)]
    properties: serde_json::Value,
}

fn classify(wrapper: SseWrapper) -> StreamEvent {
    let props = wrapper.properties;

    match wrapper.type_.as_str() {
        "server.connected" => StreamEvent::Connected,

        "session.idle" => StreamEvent::Idle,

        "session.error" => StreamEvent::SessionError {
            message: props.to_string(),
        },

        "message.part.updated" => {
            let is_text = props
                .pointer("/part/type")
                .and_then(|v| v.as_str())
                .map(|t| t == "text")
                .unwrap_or(false);

            if !is_text {
                return StreamEvent::Connected;
            }

            let text = props
                .pointer("/part/text")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            StreamEvent::TextDelta { text }
        }

        _ => StreamEvent::Connected,
    }
}
```

Three details in that code are worth pausing on, because each one is a trap:

**`Ok(0) => break`.** `read_line` returns the number of bytes read, and `0`
means end of stream. This is the loop's exit condition; forgetting it is an
infinite loop.

**`strip_prefix("data: ")`.** SSE frames look like `data: {...}` followed by a
blank line. Lines that do not start with `data: ` (including the blank
separator) are skipped by `continue`. Verified against a live `opencode serve`
1.18.31: the stream sends
`data: {"id":...,"type":"server.connected","properties":{}}` frames exactly in
this shape.

**`message.part.updated` has no `delta` field.** The part carries the full
accumulated `text` so far, not an increment. So `TextDelta` should be read as
"here is the current text of this part," which is why 05a's `apply_event`
assigns rather than appends.

> **Rust concept:** `let Some(x) = ... else { continue };`
> A `let`-`else` binding. If the pattern does not match, the `else` block runs and must diverge (`continue`, `break`, `return`, or `panic!`). It is the idiomatic way to skip uninteresting cases early.

> **Rust concept:** `cx.background_executor().spawn(...)`
> Runs a task on a thread pool allowed to block on I/O. `.detach()` keeps it alive, same as Phase 03. Note this closure takes no `this` — it talks to the UI only through the channel, never by touching the component.

> **Rust concept:** `serde_json::Value` and `pointer()`
> `Value` is parsed JSON with a shape you do not declare up front. `pointer("/part/type")` walks a JSON-path-like string and returns `Option<&Value>`. Useful when one endpoint returns many event shapes and you only care about a few fields of each.

---

## Verify

1. `opencode serve` running.
2. `cargo run`
3. The status bar should reach `Ready — ses_…` as before. Behind the scenes
   the reader is now connected to `/event`.
4. Optional debugging aid: temporarily add `println!("{event:?}");` at the top
   of `apply_event` to watch `server.connected` and periodic
   `server.heartbeat` events arrive in the terminal.

Text will not stream into the transcript yet unless Phase 04's placeholder
logic is already wired — full end-to-end verification happens after 05c.

## Common pitfall

If the terminal prints `Disconnected: ...` shortly after startup:

- Check `opencode serve` is still running.
- The error chain names the failing step (`GET /event` vs read errors).

---

Once this compiles and the reader connects, move to
**Phase 05c: Wire it end-to-end and verify streaming**.
