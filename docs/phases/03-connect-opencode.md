# Phase 03 — Connect to OpenCode

## What you will build
The app reads an environment variable, installs a real HTTP client, checks that `opencode serve` is running, and creates a session on startup.

## Concepts you will learn
- Adding dependencies in `Cargo.toml`.
- Creating a module with a subdirectory (`src/client/`).
- `async` / `await` in Rust.
- `Option` and `Result` error handling.
- GPUI's async tasks and how a result gets back into the UI.
- Reading environment variables.

## Files to touch
- `Cargo.toml`
- `src/client/types.rs` (new file)
- `src/client/mod.rs` (new file)
- `src/main.rs`
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
reqwest_client = { git = "https://github.com/egoist/zed", branch = "waku-webview" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
```

> **Rust concept:** `features`
> Crates can have optional parts that you switch on. `serde = { features = ["derive"] }` turns on the derive macros, which is what lets `#[derive(Deserialize)]` work.

**Why not `reqwest` and `tokio`?** The obvious move is to add `reqwest` for HTTP and `tokio` for async. Don't. GPUI already ships an HTTP abstraction and its own async executor, and `reqwest_client` is the adapter that plugs the two together. Adding `reqwest` directly would give you a second, disconnected HTTP stack.

Notice the four new crates are all *already* in `Cargo.lock` — `gpui` pulls them in transitively. Adding them as direct dependencies unlocks their APIs without pulling in anything new.

> **Rust concept:** a *direct* dependency vs a *transitive* one
> A transitive dependency exists in the build graph but is not visible to your code. You cannot write `use serde::...` unless `serde` is listed in your own `Cargo.toml`. The crate is already compiled; you are just asking for permission to name it.

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
> Maps the Rust field `project_id` to the JSON key `projectID`. Rust naming convention is `snake_case`; the API uses `camelCase`. The attribute bridges the two without renaming your field.

> **Rust concept:** `#[derive(Serialize)]` as well as `Deserialize`
> `Health` only ever arrives from the server, so it needs `Deserialize` alone. `Session` is both received and (later) sent, so it derives both.

---

## Step 3: Create `src/client/mod.rs`

```rust
pub mod types;

use std::sync::Arc;

use anyhow::{Context, Result};
use futures::AsyncReadExt;
use gpui::http_client::{AsyncBody, HttpClient, Json, Url};

pub use types::{Health, Session};

#[derive(Clone)]
pub struct OpencodeClient {
    http: Arc<dyn HttpClient>,
    base: Url,
}

impl OpencodeClient {
    pub fn new(base: Url, http: Arc<dyn HttpClient>) -> Self {
        Self { http, base }
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base
            .join(path)
            .with_context(|| format!("join {path} onto {}", self.base))
    }

    pub async fn health(&self) -> Result<Health> {
        let url = self.url("/global/health")?;
        let response = self
            .http
            .get(url.as_str(), AsyncBody::empty(), true)
            .await
            .context("GET /global/health")?;

        if !response.status().is_success() {
            anyhow::bail!("GET /global/health -> {}", response.status());
        }

        let mut body = String::new();
        response
            .into_body()
            .read_to_string(&mut body)
            .await
            .context("read health body")?;

        serde_json::from_str(&body).context("parse health JSON")
    }

    pub async fn create_session(&self, title: &str) -> Result<Session> {
        #[derive(serde::Serialize)]
        struct NewSession<'a> {
            title: &'a str,
        }

        let url = self.url("/session")?;
        let payload = NewSession { title };
        let response = self
            .http
            .post_json(url.as_str(), Json(&payload).into())
            .await
            .context("POST /session")?;

        if !response.status().is_success() {
            anyhow::bail!("POST /session -> {}", response.status());
        }

        let mut body = String::new();
        response
            .into_body()
            .read_to_string(&mut body)
            .await
            .context("read session body")?;

        serde_json::from_str(&body).context("parse session JSON")
    }
}
```

A few things to notice, because each one is a decision:

**`Arc<dyn HttpClient>`, not a concrete type.** `HttpClient` is a trait. GPUI hands you a trait object so the app does not care which implementation is behind it. `dyn` means "some type implementing this trait, decided at runtime." `Arc` is a reference-counted pointer so several owners can share one client. In TypeScript this is an interface-typed value; there is no `dyn` keyword because interfaces are structural.

**The client does not create the HTTP client.** It receives one. That is dependency injection, and it is why `OpencodeClient::new` cannot fail and returns `Self` rather than `Result<Self>`.

**No auth.** The local `opencode serve` needs none. Basic auth is deferred to a later phase rather than carried as dead code.

> **Rust concept:** `Result<T>`
> A value that is either `Ok(T)` (success) or `Err(error)` (failure). The `?` operator propagates errors upward, like `throw`.

> **Rust concept:** `.context(...)` and `anyhow`
> `anyhow::Context` attaches a human-readable message to an error as it travels up. Without it you get "connection refused"; with it you get "GET /global/health: connection refused". The `{e:#}` format in later steps prints that whole chain.

> **Rust concept:** `async` / `await`
> Marks a function as asynchronous and pauses it at `.await` until the operation finishes. Like JavaScript `async/await`.

> **Rust concept:** `response.into_body()`
> `into_` means "consume and convert." The response is taken apart, and its body is all that remains.

> **Rust concept:** `read_to_string(&mut body)`
> `&mut` here is a *mutable borrow*: the function writes into `body` without owning it. In JavaScript you would pass an array and push into it.

---

## Step 4: Register the client module

Add this line to `src/main.rs`:

```rust
mod client;
```

---

## Step 5: Install a real HTTP client — do not skip this

This is the step that silently ruins the phase if you miss it.

`gpui_platform::application()` installs a stub HTTP client. Every request through it fails at runtime with `No HttpClient available`. **There is no compiler error** — the types all line up, and the app runs fine until it tries to talk to the network.

Replace `main` in `src/main.rs` with:

```rust
use std::sync::Arc;

use gpui::*;
use gpui_platform::application;
use reqwest_client::ReqwestClient;

fn main() {
    application()
        .with_http_client(Arc::new(ReqwestClient::new()))
        .run(|cx: &mut App| {
            cx.bind_keys([KeyBinding::new("enter", SendPrompt, Some(("KakuApp")))]);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(900.0), px(640.0)),
                        cx,
                    ))),
                    window_min_size: Some(size(px(600.0), px(400.0))),
                    ..Default::default()
                },
                |window, cx| {
                    let app = KakuApp::new(window, cx);
                    let focus = app.focus_handle(cx).clone();
                    window.focus(&focus, cx);
                    app
                },
            )
            .expect("failed to open main window");
        });
}
```

The change is the two lines on `application()`. `.with_http_client(...)` returns the `Application` back, so it chains before `.run(...)`.

> **Rust concept:** builder pattern
> `Application` is configured by chaining methods that each return `Self`. It is the same shape as `div().flex().gap(...)` from Phases 01 and 02.

> **Rust concept:** `Arc::new(...)`
> Wraps a value in a reference-counted pointer. `with_http_client` wants to share one client across the app, and `Arc` is how Rust shares ownership. React analogy: passing one shared object down through context instead of constructing a new one per component.

**`ReqwestClient` contains a tokio runtime.** You will not write `tokio` anywhere. The client owns its runtime internally and runs its own requests on it. That is the whole reason to use this adapter instead of wiring up an HTTP client by hand.

---

## Step 6: Store session and client in `KakuApp`

In `src/app.rs`, add imports:

```rust
use crate::client::{OpencodeClient, Session};
use gpui::http_client::Url;
```

`Url` is not in `gpui::*`. It comes from the `http_client` module that `gpui` re-exports, and it is the same `Url` type `OpencodeClient::new` expects — which is why it is imported from there rather than from a `url` crate.

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
> A value that may or may not exist. `Some(T)` = exists, `None` = missing. Like `T | null` in TypeScript, except the compiler forces you to handle the `None` case before you can read the value.

---

## Step 7: Connect on startup

In `src/app.rs`, add a `connect` method:

```rust
impl KakuApp {
    fn connect(&mut self, cx: &mut Context<Self>) {
        let base = std::env::var("KAKU_GUI_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:4096".to_string());

        let url = match Url::parse(&base) {
            Ok(u) => u,
            Err(e) => {
                self.status = Status::Error(format!("invalid URL: {e}"));
                cx.notify();
                return;
            }
        };

        let client = OpencodeClient::new(url, cx.http_client());

        cx.spawn(async move |this, cx| {
            let result = async {
                client.health().await?;
                let session = client.create_session("kaku-gui").await?;
                Ok::<_, anyhow::Error>((client, session))
            }
            .await;

            let _ = this.update(cx, |this, cx| {
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
            });
        })
        .detach();
    }
}
```

Note that the closure binds the component as `this`, not `self`. Inside the spawned task, `self` does not exist — writing `self.status = ...` there fails with `error[E0425]: cannot find value 'self' in this scope`. The task only has `this`, a `WeakEntity<KakuApp>`.

That is the whole point of `this.update(cx, ...)`: the async task cannot touch the component directly, so it asks the entity to run a closure that does.

Call `connect` at the end of `KakuApp::new`:

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

> **Rust concept:** `cx.http_client()`
> Returns the client you installed in Step 5, as `Arc<dyn HttpClient>`. It is defined on `App`, and `Context<T>` derefs to `App`, so it is reachable from both.

> **Rust concept:** `cx.spawn(async move |this, cx| { ... })`
> Starts an async task tied to this component. `this` is a `WeakEntity<Self>` — a handle that does not keep the component alive. The `async move` captures `client` by value so the task owns it.
>
> **Use exactly this closure shape.** The similar-looking `cx.spawn(|this, cx| async move { ... })` does not compile (`E0282: type annotations needed`). The async-closure form is the one the GPUI examples use.

> **Rust concept:** `.detach()`
> A `Task` is cancelled when dropped. `.detach()` says "let this run to completion, I am not holding the handle." Without it, the task would be dropped immediately and the request would never happen.

> **Rust concept:** `this.update(cx, |this, cx| { ... })`
> Schedules a closure to run against the component on the UI thread. Returns a `Result`, because the component may already be gone — hence the leading `let _ =`. Network work happens off the UI thread; the state change happens on it.

> **Rust concept:** `Ok::<_, anyhow::Error>((client, session))`
> The turbofish `::<_, anyhow::Error>` names the types the compiler cannot infer: the success type is inferred from the tuple, and the error type is stated explicitly. Without it the `?` inside the async block has no error type to convert into.

---

## Step 8: Show connection status

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

> **Rust concept:** `.as_ref().map(...)`
> `as_ref()` turns `Option<Session>` into `Option<&Session>` so nothing is moved. `map` transforms the value if present, and `unwrap_or_else` supplies the fallback.

> **Rust concept:** `.unwrap_or_else(|| ...)`
> Like `.unwrap_or(...)` except the fallback is a function, so the default string is only built when it is actually needed.

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
3. The status bar should change from "Ready — not connected" to "Ready — ses_…".

## Common pitfall

If the status bar shows an error containing `No HttpClient available`, Step 5 was skipped — `.with_http_client(...)` is missing from `main.rs`. The full message reads `connect: GET /global/health: No HttpClient available`.

If it says `invalid URL` or a connection error, check that `opencode serve` is running on `http://127.0.0.1:4096`.

---

Once the status bar shows a session ID, move to **Phase 04: Send Prompts**.