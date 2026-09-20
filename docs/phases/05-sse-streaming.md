# Phase 05 — SSE Streaming

## What you will build
Listen to OpenCode's `/event` endpoint and stream assistant text into the UI in real time.

## Concepts you will learn
- Server-Sent Events (SSE) parsing.
- Running a background reader task.
- Sending events from a background task into the UI via a channel.
- Draining the channel inside `render`.

## Files to touch
- `src/client/mod.rs`
- `src/app.rs`

---

## Step 1: Add an event-stream method to the client

In `src/client/mod.rs`, add `Response` to the existing `gpui::http_client` import:

```rust
use gpui::http_client::{AsyncBody, HttpClient, Json, Response, Url};
```

And the method:

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

Note what this returns: the **unread response**, not a parsed body. SSE is an endless stream, so you cannot `read_to_string` it — that would wait forever. The caller reads it line by line instead.

That is also why the client needs no new imports beyond `Response`: the streaming happens in `app.rs`, and the client only hands over the response.

> **Rust concept:** why the import list changed
> `Response` and `AsyncBody` are the same types Phase 03 already used, but `Response` was not needed by name before. A function that *returns* a type must name it in its signature.

---

## Step 2: Add a `StreamEvent` enum

In `src/app.rs`, add:

```rust
#[derive(Clone, Debug)]
pub enum StreamEvent {
    Connected,
    Idle,
    Disconnected(String),
    TextDelta { text: String },
    SessionError { message: String },
}
```

> **Rust concept:** enum variants carrying data
> `Disconnected(String)` and `TextDelta { text: String }` are the same idea written two ways — a tuple variant and a struct variant. The struct form names its fields, which reads better once a variant has more than one. `Disconnected(String)` is fine with one field.

---

## Step 3: Add an event channel to `KakuApp`

Add a field:

```rust
pub struct KakuApp {
    ...
    events: Option<std::sync::mpsc::Receiver<StreamEvent>>,
}
```

Initialize it as `None`:

```rust
events: None,
```

> **Rust concept:** `std::sync::mpsc::Receiver<T>`
> The receiving half of a channel. `mpsc` means "multi-producer, single-consumer": many senders, one receiver. The background task holds the `Sender`, the UI holds the `Receiver`. This is the standard-library channel, so it adds no dependency.

---

## Step 4: Start the SSE reader after connecting

In `connect`, in the `Ok((client, session))` branch, start the reader:

```rust
Ok((client, session)) => {
    this.client = Some(client);
    this.session = Some(session);
    this.status = Status::Idle;
    this.start_sse_reader(cx);
}
```

Then add these imports at the top of `src/app.rs` — the reader runs here, not in the client:

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

**`Ok(0) => break`.** `read_line` returns the number of bytes read, and `0` means end of stream. This is the loop's exit condition; forgetting it is an infinite loop.

**`strip_prefix("data: ")`.** SSE frames look like `data: {...}` followed by a blank line. Lines that do not start with `data: ` (including the blank separator) are skipped by `continue`.

**`message.part.updated` has no `delta` field.** The part carries the full accumulated `text` so far, not an increment. So `TextDelta` should be read as "here is the current text of this part," and the UI assigns it rather than appending. Step 5 does exactly that.

> **Rust concept:** `let Some(x) = ... else { continue };`
> A `let`-`else` binding. If the pattern does not match, the `else` block runs and must diverge (`continue`, `break`, `return`, or `panic!`). It is the idiomatic way to skip uninteresting cases early.

> **Rust concept:** `cx.background_executor().spawn(...)`
> Runs a task on a thread pool allowed to block on I/O. `.detach()` keeps it alive, same as Phase 03.

---

## Step 5: Drain events in `render`

Update `render` to drain the channel at the top:

```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    while let Some(event) = self.events.as_ref().and_then(|rx| rx.try_recv().ok()) {
        self.apply_event(event, cx);
    }

    ...
}
```

Add `apply_event`:

```rust
impl KakuApp {
    fn apply_event(&mut self, event: StreamEvent, cx: &mut Context<Self>) {
        match event {
            StreamEvent::Connected => {}

            StreamEvent::Idle => {
                self.status = Status::Idle;
                self.streaming_idx = None;
            }

            StreamEvent::Disconnected(why) => {
                if matches!(self.status, Status::Busy) {
                    self.status = Status::Error(format!("disconnected: {why}"));
                    self.streaming_idx = None;
                }
            }

            StreamEvent::SessionError { message } => {
                self.status = Status::Error(message);
                self.streaming_idx = None;
            }

            StreamEvent::TextDelta { text } => {
                if let Some(idx) = self.streaming_idx {
                    if let Some(message) = self.messages.get_mut(idx) {
                        message.text = text;
                    }
                }
            }
        }

        cx.notify();
    }
}
```

`message.text = text` **assigns** rather than appends, because the event carries the full text of the part so far. Appending would duplicate everything on every frame.

> **Rust concept:** `self.events.as_ref().and_then(|rx| rx.try_recv().ok())`
> `as_ref()` borrows the `Option` instead of consuming it, so the receiver stays in `self`. `try_recv()` returns a `Result`, and `.ok()` converts it to `Option`, which `and_then` flattens. The loop ends when the channel is empty.
>
> This avoids the `self.events.take()` dance: because the borrow ends each iteration, `self` is free to be mutated by `apply_event`.

> **Rust concept:** `matches!(self.status, Status::Busy)`
> A macro that answers "does this value match this pattern?" without binding anything. Useful when you care about the variant but not its contents.

> **Rust concept:** `self.messages.get_mut(idx)`
> Returns `Option<&mut DisplayMessage>` — `Some` if the index is in range, `None` otherwise. Safer than `self.messages[idx]`, which panics on a stale index.

---

## Verify

1. `opencode serve` running.
2. `cargo run`
3. Type a message and press Enter.
4. The assistant placeholder should fill with text as it streams in.
5. When the response finishes, the status bar returns to "Ready."

The server sends `server.connected` immediately and `server.heartbeat` periodically. Both are handled, and neither should add text to the transcript.

## Common pitfall

If the assistant message stays empty:

- Check the status bar shows a session ID, which means the SSE reader started.
- Add `println!("{event:?}");` at the top of `apply_event` to see what actually arrives.
- If you see `message.part.updated` events but no text, the part `type` is probably not `text` — reasoning parts and tool parts arrive through the same event with a different `type`.

---

Once streaming works, move to **Phase 06: Abort, Scroll, and Status Polish**.