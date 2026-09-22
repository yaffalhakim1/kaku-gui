# Phase 03b — Install the HTTP client and connect on startup

> This phase was split from `03-connect-opencode.md` (2026-09-22) for pacing.
> This is **part 2 of 3**. Prerequisite: Phase 03a compiles with 0 errors.
>
> **Status: in progress 2026-09-22.** Step 1 and Step 2 are done (verified,
> 0 errors). Step 3 remains.

## What you will build

The GPUI half of the connection: install a real HTTP client, store the client
and session on `KakuApp`, and run `connect` as an async task on startup.

## Concepts you will learn
- `Option<T>` as a type-level fact.
- GPUI's async tasks (`cx.spawn`).
- `WeakEntity` and why `this` replaces `self` inside a task.
- `.detach()` and task cancellation.
- The turbofish (`Ok::<_, E>`).

## Files to touch
- `src/main.rs`
- `src/app.rs`

---

## Step 1: Install a real HTTP client — do not skip this

This is the step that silently ruins the phase if you miss it.

`gpui_platform::application()` installs a stub HTTP client. Every request
through it fails at runtime with `No HttpClient available`. **There is no
compiler error** — the types all line up, and the app runs fine until it tries
to talk to the network.

In `src/main.rs`, add two imports:

```rust
use std::sync::Arc;
use reqwest_client::ReqwestClient;
```

Then replace `application().run(...)` with the chained form:

```rust
application()
    .with_http_client(Arc::new(ReqwestClient::new()))
    .run(|cx: &mut App| {
        // ... existing body unchanged ...
    });
```

The rest of the `run` body stays exactly as it is.

> **Rust concept:** builder pattern
> `Application` is configured by chaining methods that each return `Self`. It is the same shape as `div().flex().gap(...)` from Phase 01.

> **Rust concept:** `Arc::new(...)`
> Wraps a value in a reference-counted pointer. `with_http_client` wants to share one client across the app, and `Arc` is how Rust shares ownership. React analogy: passing one shared object down through context instead of constructing a new one per component.

**`ReqwestClient` contains a tokio runtime.** You will not write `tokio`
anywhere. The client owns its runtime internally and runs its own requests on
it. That is the whole reason to use this adapter instead of wiring up an HTTP
client by hand.

---

## Step 2: Store session and client in `KakuApp`

In `src/app.rs`, add imports:

```rust
use crate::client::{OpencodeClient, Session};
use gpui::http_client::Url;
```

`Url` is not in `gpui::*`. It comes from the `http_client` module that `gpui`
re-exports, and it is the same `Url` type `OpencodeClient::new` expects — which
is why it is imported from there rather than from a `url` crate.

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

## Step 3: Connect on startup

In `src/app.rs`, add a `connect` method in its own `impl` block:

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

Note that the closure binds the component as `this`, not `self`. Inside the
spawned task, `self` does not exist — writing `self.status = ...` there fails
with `error[E0425]: cannot find value 'self' in this scope`. The task only has
`this`, a `WeakEntity<KakuApp>`.

That is the whole point of `this.update(cx, ...)`: the async task cannot touch
the component directly, so it asks the entity to run a closure that does.

Call `connect` at the end of `KakuApp::new`. The `cx.new(...)` call becomes a
`let app = ...` binding, then two lines follow:

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

Three concrete edits to `new`:

1. `cx.new(|cx| {` becomes `let app = cx.new(|cx| {`
2. after the closing `});` of `cx.new`, add `app.update(cx, |this, cx| this.connect(cx));`
3. the final line becomes `app`

> **Rust concept:** why the intermediate `app`
> `connect` needs `&mut Context<Self>`, but inside the `cx.new` closure the
> component is still being *built* — there is no `self` to borrow yet. So:
> build the entity handle first, then `app.update(cx, ...)` runs code against
> the finished component.

> **Rust concept:** `cx.http_client()`
> Returns the client you installed in Step 1, as `Arc<dyn HttpClient>`. It is defined on `App`, and `Context<T>` derefs to `App`, so it is reachable from both.

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

## Verify

```bash
cargo check
```

**Expect 0 errors and ~5 warnings.** The Phase 03a warnings for
`OpencodeClient`, `health`, `create_session`, and the struct warnings all
disappear — `connect` uses them. Remaining: `System` never constructed
(Phase 07), `Busy`/`Error` never constructed (03c/04), `surface` never read
(Phase 01), and the cosmetic parens in `main.rs`.

`method 'connect' is never used` appears until Step 3's `app.update(cx, ...)`
line is in place.

---

## Common pitfall

`No HttpClient available` means Step 1 was skipped — `.with_http_client(...)`
is missing from `main.rs`.

Once this compiles clean, move to **Phase 03c: Show the session ID**.
