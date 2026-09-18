# Phase 05 — SSE Streaming

## What you will build
Listen to OpenCode's `/event` endpoint and stream assistant tokens into the UI in real time.

## Concepts you will learn
- Server-Sent Events (SSE) parsing.
- Running a background reader task.
- Sending events from a background task into the UI via a channel.
- Draining the channel inside `render`.

## Files to touch
- `src/app.rs`
- `src/client/mod.rs`

---

## Step 1: Add `http_get` to the client

In `src/client/mod.rs`, add:

```rust
use reqwest::{Client, RequestBuilder, Url};
```

And the method:

```rust
pub fn http_get(&self, url: Url) -> RequestBuilder {
    self.http.get(url)
}
```

This exposes the raw request builder so the SSE reader can stream bytes.

---

## Step 2: Add a `StreamEvent` enum

In `src/app.rs`, add:

```rust
#[derive(Clone, Debug)]
pub enum StreamEvent {
    Connected,
    Idle,
    Disconnected(String),
    PartUpdated { text: String, delta: Option<String> },
    SessionError { message: String },
}
```

---

## Step 3: Add an event channel to `KakuApp`

Add a field:

```rust
pub struct KakuApp {
    ...
    events: Option<std::sync::mpsc::Receiver<StreamEvent>>,
}
```

Initialize:

```rust
events: None,
```

---

## Step 4: Start the SSE reader after connecting

In `connect`, after `this.client = Some(client); this.session = Some(session);`, start the SSE reader.

Add this inside the `Ok((client, session))` branch:

```rust
this.start_sse_reader(cx);
```

Then add the method:

```rust
impl KakuApp {
    fn start_sse_reader(&mut self, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else { return };

        let (tx, rx) = std::sync::mpsc::channel();
        self.events = Some(rx);

        cx.background_executor().spawn(async move {
            let url = match client.base_url().join("/event") {
                Ok(u) => u,
                Err(_) => return,
            };

            let resp = match client.http_get(url).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(StreamEvent::Disconnected(format!("sse: {e}")));
                    return;
                }
            };

            if !resp.status().is_success() {
                let _ = tx.send(StreamEvent::Disconnected("non-2xx status".to_string()));
                return;
            }

            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(next) = stream.next().await {
                let chunk = match next {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(StreamEvent::Disconnected(format!("sse: {e}")));
                        return;
                    }
                };
                buf.extend_from_slice(&chunk);

                while let Some(idx) = find_subsequence(&buf, b"\n\n") {
                    let raw: Vec<u8> = buf.drain(..idx + 2).collect();
                    let Ok(s) = std::str::from_utf8(&raw) else { continue };
                    let json: String = s
                        .lines()
                        .filter_map(|l| l.strip_prefix("data: "))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if json.is_empty() {
                        continue;
                    }
                    let Ok(wrapper) = serde_json::from_str::<SseWrapper>(&json) else {
                        continue;
                    };
                    let ev = classify(wrapper);
                    if tx.send(ev).is_err() {
                        return;
                    }
                }
            }

            let _ = tx.send(StreamEvent::Disconnected("closed".to_string()));
        }).detach();
    }
}
```

Add the helper types and functions at the bottom of `src/app.rs`:

```rust
#[derive(Debug, serde::Deserialize)]
struct SseWrapper {
    #[serde(rename = "type")]
    type_: String,
    #[serde(default)]
    properties: serde_json::Value,
}

fn classify(w: SseWrapper) -> StreamEvent {
    let t = w.type_.as_str();
    if t == "server.connected" {
        return StreamEvent::Connected;
    }
    let props = w.properties;
    match t {
        "session.idle" => {
            let id = props
                .get("sessionID")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let _ = id;
            StreamEvent::Idle
        }
        "session.error" => StreamEvent::SessionError {
            message: props.to_string(),
        },
        "message.part.updated" => {
            let is_text = props
                .pointer("/part/type")
                .and_then(|v| v.as_str())
                .map(|s| s == "text")
                .unwrap_or(false);
            if !is_text {
                return StreamEvent::Connected;
            }
            let text = props
                .pointer("/part/text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let delta = props.get("delta").and_then(|v| v.as_str()).map(String::from);
            StreamEvent::PartUpdated { text, delta }
        }
        _ => StreamEvent::Connected,
    }
}

fn find_subsequence(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}
```

> **Rust concept:** `std::sync::mpsc::channel`
> Creates a multi-producer, single-consumer channel. The background SSE task sends events; the UI thread receives them.

> **Rust concept:** `cx.background_executor().spawn(...)`
> Runs a task on a thread pool that is allowed to do I/O. This is where network streaming belongs.

---

## Step 5: Drain events in `render`

Update `render` to drain the channel:

```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    if let Some(rx) = self.events.take() {
        while let Ok(ev) = rx.try_recv() {
            self.apply_event(ev, cx);
        }
        self.events = Some(rx);
    }

    ...
}
```

Add `apply_event`:

```rust
impl KakuApp {
    fn apply_event(&mut self, ev: StreamEvent, cx: &mut Context<Self>) {
        match ev {
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
            StreamEvent::PartUpdated { text, delta } => {
                if let Some(idx) = self.streaming_idx {
                    if idx < self.messages.len() {
                        match delta {
                            Some(d) if !d.is_empty() => self.messages[idx].text.push_str(&d),
                            _ => self.messages[idx].text = text,
                        }
                    }
                }
            }
        }
        cx.notify();
    }
}
```

> **Rust concept:** `self.events.take()`
> Temporarily removes the receiver from `self` so we can mutate `self` inside the loop without borrow conflicts. We put it back afterward.

---

## Verify

1. `opencode serve` running.
2. `cargo run`
3. Type a message and press Enter.
4. The assistant placeholder should fill with text as it streams in.
5. When the response finishes, the status bar returns to "Ready."

## Common pitfall

If the assistant message stays empty:
- Check that the SSE reader started (status bar should show session ID).
- Add a `println!("{:?}", ev);` inside `apply_event` to see what events arrive.

---

Once streaming works, move to **Phase 06: Abort, Scroll, and Status Polish**.
