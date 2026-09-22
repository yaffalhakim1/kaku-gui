# Phase 05c — Streaming end-to-end: send, stream, finish

> This phase was split from `05-sse-streaming.md` (2026-09-22) for pacing.
> This is **part 3 of 3**. Prerequisite: Phase 05b compiles with 0 errors and
> the reader connects.
>
> **Status: not started.**

## What you will build

The last wiring: `send_prompt` calls OpenCode and `streaming_idx` routes
`TextDelta` events into the assistant placeholder created in Phase 04. After
this part, the transcript fills in real time and the status bar returns to
"Ready." when the response ends.

## Concepts you will learn
- Calling an async HTTP method from an action handler (revisiting Phase 03b's patterns).
- Tracking a streaming target with `Option<usize>`.

## Files to touch
- `src/client/mod.rs`
- `src/app.rs`

---

## Step 1: Add `send_prompt` to the client

Open `src/client/mod.rs`. `Json` is already imported from Phase 03. Add this
method inside `impl OpencodeClient`:

```rust
pub async fn send_prompt(&self, session_id: &str, text: &str) -> Result<()> {
    #[derive(serde::Serialize)]
    struct PromptBody<'a> {
        parts: [TextPart<'a>; 1],
    }

    #[derive(serde::Serialize)]
    struct TextPart<'a> {
        #[serde(rename = "type")]
        type_: &'static str,
        text: &'a str,
    }

    let url = self.url(&format!("/session/{session_id}/prompt_async"))?;
    let payload = PromptBody {
        parts: [TextPart {
            type_: "text",
            text,
        }],
    };

    let response = self
        .http
        .post_json(url.as_str(), Json(&payload).into())
        .await
        .context("POST prompt_async")?;

    if !response.status().is_success() {
        anyhow::bail!("POST prompt_async -> {}", response.status());
    }

    Ok(())
}
```

> **Rust concept:** `#[serde(rename = "type")]`
> `type` is a reserved word in Rust, so the field cannot be called that. `type_` with a rename attribute is the convention for this exact situation. Phase 05b's `SseWrapper` does the same thing.

> **Rust concept:** `[TextPart<'a>; 1]`
> A fixed-size array of exactly one element. `parts` is required by the API but only ever holds one text part here, so an array states that precisely. `Vec<TextPart>` would also work.

> **Rust concept:** `&format!("/session/{session_id}/prompt_async")`
> Builds a URL path string. `format!` is like JavaScript's template literal.
>
> Note `self.url(...)` rather than `self.base.join(...)`: `url` is the private helper from Phase 03 that joins and attaches context to the error. Use it consistently so failures name the request that failed.

---

## Step 2: Add `streaming_idx` to `KakuApp`

Add a field to track which message is currently receiving the stream:

```rust
pub struct KakuApp {
    ...
    streaming_idx: Option<usize>,
}
```

Initialize it as `None` in `KakuApp::new`:

```rust
streaming_idx: None,
```

---

## Step 3: Update `send_prompt_action`

Replace the existing `send_prompt_action` with this:

```rust
fn send_prompt_action(
    &mut self,
    _: &SendPrompt,
    _window: &mut Window,
    cx: &mut Context<Self>,
) {
    let content = self.input.read(cx).content().clone();
    let text = content.to_string();
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }

    let Some(session) = self.session.as_ref() else {
        self.status = Status::Error("not connected".to_string());
        cx.notify();
        return;
    };

    let Some(client) = self.client.clone() else {
        return;
    };

    let session_id = session.id.clone();

    self.messages.push(DisplayMessage {
        role: Role::User,
        text: text.clone(),
    });
    self.messages.push(DisplayMessage {
        role: Role::Assistant,
        text: String::new(),
    });
    self.streaming_idx = Some(self.messages.len() - 1);
    self.status = Status::Busy;
    self.input.update(cx, |input, cx| input.clear(cx));
    cx.notify();

    cx.spawn(async move |this, cx| {
        let result = client.send_prompt(&session_id, &text).await;

        if let Err(e) = result {
            let _ = this.update(cx, |this, cx| {
                this.status = Status::Error(format!("send: {e:#}"));
                cx.notify();
            });
        }
    })
    .detach();
}
```

> **Rust concept:** `let Some(session) = self.session.as_ref() else { ... };`
> This is an `else` branch on `let`. If `session` is `None`, the code in `else` runs and returns early. It reads like a guard clause in JS.

> **Rust concept:** `self.client.clone()`
> `OpencodeClient` is `Clone`, and its fields are an `Arc` and a `Url` — so cloning it copies two pointers, not a connection pool. The clone exists because the async task must own what it uses, and the task outlives the `&mut self` borrow that produced it.

---

## Verify — full end-to-end

1. `opencode serve` running.
2. `cargo run`
3. Type a message and press Enter.
4. The assistant placeholder should fill with text as it streams in.
5. When the response finishes, the status bar returns to "Ready."

The server sends `server.connected` immediately and `server.heartbeat`
periodically. Both are handled (they classify to `StreamEvent::Connected`,
which is a no-op), and neither should add text to the transcript.

## Common pitfall

If the assistant message stays empty:

- Check the status bar shows a session ID, which means the SSE reader started.
- Add `println!("{event:?}");` at the top of `apply_event` to see what actually arrives.
- If you see `message.part.updated` events but no text, the part `type` is probably not `text` — reasoning parts and tool parts arrive through the same event with a different `type`.

---

Once streaming works end-to-end, **Phase 05 is complete**. Move to
**Phase 06: Abort, Scroll, and Status Polish**.
