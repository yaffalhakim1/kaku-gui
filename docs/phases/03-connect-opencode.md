# Phase 03 — Connect to OpenCode

## What you will build
The app reads environment variables, checks that `opencode serve` is running, and creates a session on startup.

## Concepts you will learn
- Adding dependencies in `Cargo.toml`.
- Creating a module with a subdirectory (`src/client/`).
- `async` / `await` in Rust.
- `Option` and `Result` error handling.
- GPUI's background executor.
- Reading environment variables.

## Files to touch
- `Cargo.toml`
- `src/client/types.rs` (new file)
- `src/client/mod.rs` (new file)
- `src/app.rs`

---

## Step 1: Add dependencies

Open `Cargo.toml` and replace the `[dependencies]` section with:

```toml
[dependencies]
gpui = { git = "https://github.com/egoist/zed", branch = "waku-webview" }
gpui_platform = { git = "https://github.com/egoist/zed", branch = "waku-webview", features = [
    "font-kit",
    "wayland",
    "x11",
] }
anyhow = "1"
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures-util = "0.3"
```

> **Rust concept:** `features`
> Crates can have optional parts. `tokio = { features = ["full"] }` enables all of tokio's capabilities, including the runtime we need for HTTP.

---

## Step 2: Create `src/client/types.rs`

Create the directory and file. This file defines the JSON shapes OpenCode returns.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct Health {
    pub healthy: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub time: SessionTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
}
```

> **Rust concept:** `#[derive(Deserialize)]`
> Tells serde how to convert JSON into this struct. It auto-generates parsing code.

> **Rust concept:** `#[serde(rename = "projectID")]`
> Maps the Rust field `project_id` to the JSON key `projectID`.

---

## Step 3: Create `src/client/mod.rs`

```rust
pub mod types;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client, Url};

pub use types::{Health, Session};

#[derive(Debug, Clone)]
pub struct OpencodeClient {
    http: Client,
    base: Url,
}

impl OpencodeClient {
    pub fn new(base: Url, username: &str, password: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        if let Some(pw) = password {
            let creds = format!("{username}:{pw}");
            let encoded = base64_encode(&creds);
            let val = HeaderValue::from_str(&format!("Basic {encoded}"))?;
            headers.insert(AUTHORIZATION, val);
        }
        let http = Client::builder()
            .default_headers(headers)
            .build()
            .context("build reqwest client")?;
        Ok(Self { http, base })
    }

    pub async fn health(&self) -> Result<Health> {
        let url = self.base.join("/global/health")?;
        let h = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<Health>()
            .await?;
        Ok(h)
    }

    pub async fn create_session(&self, title: Option<&str>) -> Result<Session> {
        let url = self.base.join("/session")?;
        let body = serde_json::json!({ "title": title });
        let s = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json::<Session>()
            .await?;
        Ok(s)
    }
}

fn base64_encode(input: &str) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}
```

> **Rust concept:** `Result<T>`
> A value that is either `Ok(T)` (success) or `Err(error)` (failure). The `?` operator propagates errors upward.

> **Rust concept:** `async` / `await`
> Marks a function as asynchronous and pauses it at `.await` until the operation finishes. Like JavaScript `async/await`.

---

## Step 4: Register the client module

Add this line to `src/main.rs`:

```rust
mod client;
```

---

## Step 5: Store session and client in `KakuApp`

In `src/app.rs`, add imports:

```rust
use crate::client::{OpencodeClient, Session};
```

Add fields:

```rust
pub struct KakuApp {
    focus_handle: FocusHandle,
    theme: Theme,
    messages: Vec<DisplayMessage>,
    status: Status,
    input: Entity<TextInput>,
    session: Option<Session>,
    client: Option<OpencodeClient>,
}
```

Initialize them as `None`:

```rust
cx.new(|cx| {
    let input = cx.new(|cx| TextInput::new(cx));

    Self {
        focus_handle: cx.focus_handle(),
        theme: Theme::dark(),
        messages: vec![...],
        status: Status::Idle,
        input,
        session: None,
        client: None,
    }
})
```

> **Rust concept:** `Option<T>`
> A value that may or may not exist. `Some(T)` = exists, `None` = missing. Like `T | null` in TypeScript.

---

## Step 6: Connect on startup

In `src/app.rs`, add a `connect` method:

```rust
impl KakuApp {
    fn connect(&mut self, cx: &mut Context<Self>) {
        let base = std::env::var("KAKU_GUI_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4096".to_string());
        let password = std::env::var("KAKU_GUI_PASSWORD")
            .ok()
            .or_else(|| std::env::var("OPENCODE_SERVER_PASSWORD").ok());
        let username = std::env::var("OPENCODE_SERVER_USERNAME")
            .unwrap_or_else(|_| "opencode".to_string());

        let url: reqwest::Url = match base.parse() {
            Ok(u) => u,
            Err(e) => {
                self.status = Status::Error(format!("invalid URL: {e}"));
                cx.notify();
                return;
            }
        };

        let client = match OpencodeClient::new(url, &username, password.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                self.status = Status::Error(format!("client: {e:#}"));
                cx.notify();
                return;
            }
        };

        cx.spawn(|this, mut cx| async move {
            let result = async {
                client.health().await?;
                let session = client.create_session(Some("kaku-gui")).await?;
                Ok::<_, anyhow::Error>((client, session))
            }
            .await;

            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    match result {
                        Ok((client, session)) => {
                            this.client = Some(client);
                            this.session = Some(session);
                            this.status = Status::Idle;
                        }
                        Err(e) => {
                            this.status = Status::Error(format!("connect: {e:#}"));
                        }
                    }
                    cx.notify();
                })
                .ok();
            })
            .ok();
        })
        .detach();
    }
}
```

Call it at the end of `KakuApp::new`:

```rust
let mut app = cx.new(|cx| { ... });
app.update(cx, |this, cx| this.connect(cx));
app
```

Wait — `KakuApp::new` currently returns `Entity<Self>` directly. We need to capture it, call connect, then return it. Change `KakuApp::new` to:

```rust
pub fn new(_window: &mut Window, cx: &mut App) -> Entity<Self> {
    let app = cx.new(|cx| {
        let input = cx.new(|cx| TextInput::new(cx));

        Self {
            focus_handle: cx.focus_handle(),
            theme: Theme::dark(),
            messages: vec![...],
            status: Status::Idle,
            input,
            session: None,
            client: None,
        }
    });

    app.update(cx, |this, cx| this.connect(cx));
    app
}
```

> **Rust concept:** `cx.spawn(...)`
> Runs an async task on GPUI's background executor. Network calls must not block the UI thread, so we run them here.

> **Rust concept:** `move`
> The closure takes ownership of `client` so it can be used inside the async task.

> **Rust concept:** `async move { ... }.await`
> The block inside `cx.spawn` is async. We `.await` the network calls.

> **Rust concept:** `this.update(cx, |this, cx| { ... })`
> After the network call finishes, we safely update the `KakuApp` entity back on the UI thread.

---

## Step 7: Show connection status

Update `render_status_bar` so it also shows the session ID when connected:

```rust
fn render_status_bar(&self, status: Status, theme: Theme) -> impl IntoElement {
    let session_id = self
        .session
        .as_ref()
        .map(|s| s.id.clone())
        .unwrap_or_else(|| "not connected".to_string());

    let label = match status {
        Status::Idle => format!("Ready — {session_id}"),
        Status::Busy => "Thinking…".to_string(),
        Status::Error(ref e) => format!("Error: {e}"),
    };

    div()
        .h(px(24.0))
        .px(px(12.0))
        .flex()
        .flex_row()
        .items_center()
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .text_size(px(11.0))
                .text_color(theme.muted)
                .child(label),
        )
}
```

---

## Verify

1. Start opencode:
   ```bash
   opencode serve
   ```
2. In another terminal:
   ```bash
   cargo run
   ```
3. The status bar should change from "Ready — not connected" to "Ready — <session-id>".

## Common pitfall

If you see "invalid URL" or "connect: ...", check that `opencode serve` is running on `http://127.0.0.1:4096`.

---

Once the status bar shows a session ID, move to **Phase 04: Send Prompts**.
