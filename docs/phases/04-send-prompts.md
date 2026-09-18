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

Open `src/client/mod.rs` and add this method inside `impl OpencodeClient`:

```rust
pub async fn send_prompt(&self, session_id: &str, text: &str) -> Result<()> {
    let url = self.base.join(&format!("/session/{session_id}/prompt_async"))?;
    let body = serde_json::json!({
        "parts": [{ "type": "text", "text": text }],
    });
    self.http
        .post(url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
```

> **Rust concept:** `&format!("/session/{session_id}/prompt_async")`
> Builds a URL path string. `format!` is like JavaScript's template literal.

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

    cx.spawn(|this, mut cx| async move {
        let result = client.send_prompt(&session_id, &text).await;

        cx.update(|cx| {
            this.update(cx, |this, _cx| {
                if let Err(e) = result {
                    this.status = Status::Error(format!("send: {e:#}"));
                }
            })
            .ok();
        })
        .ok();
    })
    .detach();
}
```

> **Rust concept:** `let Some(session) = self.session.as_ref() else { ... };`
> This is an `else` branch on `let`. If `session` is `None`, the code in `else` runs and returns early.

> **Rust concept:** `clone()` on `Option<OpencodeClient>`
> We clone the client so the async task can own it. `OpencodeClient` is `Clone`, so this is cheap.

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
