# Phase 03a — Client crate: types and HTTP methods

> This phase was split from `03-connect-opencode.md` (2026-09-22) for pacing.
> This is **part 1 of 3**. The original file is now a pointer stub.
>
> **Status: completed 2026-09-22.** Compiles with 0 errors.

## What you will build

The pure-Rust half of the OpenCode client: JSON types, URL building, and two
async methods (`health`, `create_session`). Nothing touches the network yet
and no GPUI async appears — that is Phase 03b.

## Concepts you will learn
- Adding dependencies in `Cargo.toml` (direct vs transitive).
- Creating a module with a subdirectory (`src/client/`).
- `async` / `await` in Rust.
- `Result` error handling with `anyhow`.
- Borrowing with `&self`.

## Files to touch
- `Cargo.toml`
- `src/client/types.rs` (new file)
- `src/client/mod.rs` (new file)
- `src/main.rs`

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

> **Dependency fact (corrected 2026-09-22):** `reqwest_client` is **not** already
> in `Cargo.lock`. Adding it brings roughly 78 new crates (tokio, hyper, rustls,
> h2, tower) and the first `cargo check` takes about 1.5 minutes. After that
> first build, subsequent checks are fast. `serde`, `serde_json`, `futures`,
> and `anyhow` ARE already in the lock via `gpui` — those four are free.

> **Rust concept:** a *direct* dependency vs a *transitive* one
> A transitive dependency exists in the build graph but is not visible to your code. You cannot write `use serde::...` unless `serde` is listed in your own `Cargo.toml`. The crate is already compiled; you are just asking for permission to name it.

**Run `cargo check` now, before typing any code.** The first one is the slow
one. Do it while your code is still trivial so any dependency problem surfaces
early.

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

The real API response also contains `slug`, `cost`, `tokens`, and `path`.
Serde ignores JSON fields your struct does not declare, so you parse only what
you need.

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

**`Arc<dyn HttpClient>`, not a concrete type.** `HttpClient` is a trait (an
interface). GPUI hands you a trait object so the app does not care which
implementation is behind it. `dyn` means "some type implementing this trait,
decided at runtime." `Arc` is a reference-counted pointer so several owners can
share one client. In TypeScript this is an interface-typed value; there is no
`dyn` keyword because interfaces are structural.

**The client does not create the HTTP client.** It receives one. That is
dependency injection, and it is why `OpencodeClient::new` cannot fail and
returns `Self` rather than `Result<Self>`. `new` takes no `self` parameter —
it *creates* the value, so there is nothing to borrow yet. That is why it is
called as `OpencodeClient::new(...)` with `::`, like a static method.

**No auth.** The local `opencode serve` needs none. Basic auth is deferred to a
later phase rather than carried as dead code.

**The request body is a private struct**, not a `json!` macro call. A typo'd
field name is a compile error instead of a runtime 400.

> **Rust concept:** `Result<T>`
> A value that is either `Ok(T)` (success) or `Err(error)` (failure). The `?` operator propagates errors upward, like `throw`.

> **Rust concept:** `.context(...)` and `anyhow`
> `anyhow::Context` attaches a human-readable message to an error as it travels up. Without it you get "connection refused"; with it you get "GET /global/health: connection refused". The `{e:#}` format in later steps prints that whole chain.

> **Rust concept:** `anyhow::bail!`
> Shorthand for "return `Err(...)` from this function right now."

> **Rust concept:** `async` / `await`
> Marks a function as asynchronous and pauses it at `.await` until the operation finishes. Like JavaScript `async/await`. A function containing `.await` must itself be `async` — async is contagious.

> **Rust concept:** `response.into_body()`
> `into_` means "consume and convert." The response is taken apart, and its body is all that remains.

> **Rust concept:** `read_to_string(&mut body)`
> `&mut` here is a *mutable borrow*: the function writes into `body` without owning it. In JavaScript you would pass an array and push into it.

---

## Step 4: Register the client module

Add this line to `src/main.rs`, next to the other `mod` declarations:

```rust
mod client;
```

Without it, Rust never compiles the folder and `crate::client` will not exist
(`E0433: use of undeclared crate or module 'client'`).

---

## Verify

```bash
cargo check
```

**Expect 0 errors and 9 warnings.** All 9 are expected at this point:

| Warning | Why it is expected |
|---|---|
| `unnecessary parentheses around function argument` | `Some(("KakuApp"))` in `main.rs` is cosmetic; leave it. |
| `variant 'System' is never constructed` | Used in Phase 07. |
| `variants 'Busy' and 'Error' are never constructed` | Used in 03b/03c. |
| `field 'surface' is never read` | Phase 01 leftover. |
| `struct 'OpencodeClient' is never constructed` | Nothing calls it until 03b. |
| `associated items 'new', 'base_url', 'url', 'health', 'create_session' are never used` | Same reason. |
| `structs 'Health', 'Session', 'SessionTime' never constructed` | Same reason. |

Warnings are not failures. Nothing to fix.

---

Once this compiles clean, move to **Phase 03b: Install the HTTP client and
connect on startup**.
