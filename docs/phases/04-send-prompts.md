# Phase 04 — Send Prompts

## What you will build
When you press Enter, the app sends your message to OpenCode and pre-creates an empty assistant placeholder.

## Concepts you will learn
- Calling an async HTTP method from an action handler.
- Appending to a `Vec`.
- Keeping track of the "streaming" message index.
- Updating child state from a parent.

## Files to touch
- `src/client/mod.rs`
- `src/app.rs`

---

## Step 1: Add `send_prompt` to the client

Open `src/client/mod.rs`. `Json` and `Context` are already imported from Phase 03, so add this method inside `impl OpencodeClient`:

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
> `type` is a reserved word in Rust, so the field cannot be called that. `type_` with a rename attribute is the convention for this exact situation. Phase 05's `SseWrapper` does the same thing.

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
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
    status: Status,
    input: Entity<TextInput>,
    session: Option<Session>,
    client: Option<OpencodeClient>,
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
> This is an `else` branch on `let`. If `session` is `None`, the code in `else` runs and returns early.

> **Rust concept:** `self.client.clone()`
> `OpencodeClient` is `Clone`, and its fields are an `Arc` and a `Url` — so cloning it copies two pointers, not a connection pool. The clone exists because the async task must own what it uses, and the task outlives the `&mut self` borrow that produced it.

---

## Verify

1. Make sure `opencode serve` is running.
2. `cargo run`
3. Type a message and press Enter.
4. You should see:
   - Your message appears as a user message.
   - An empty assistant message appears below it.
   - The status bar says "Thinking…".

The assistant message will stay empty until Phase 05, when we wire up SSE streaming.

## Common pitfall

If you get "not connected", it means Phase 03 did not finish successfully. Check the status bar and the terminal output.

---

Once user messages and the assistant placeholder appear, move to **Phase 05: SSE Streaming**.
