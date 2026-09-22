# Phase 03c — Show the session ID in the status bar

> This phase was split from `03-connect-opencode.md` (2026-09-22) for pacing.
> This is **part 3 of 3**. Prerequisite: Phase 03b compiles with 0 errors and
> `connect` is wired into `KakuApp::new`.
>
> **Status: completed 2026-09-22.**

## What you will build

One method: `render_status_bar` reads the stored session and shows its id.
This is the visible payoff of the whole Phase 03 — "connected" becomes
something you can see.

## Concepts you will learn
- The `Option` ladder: `as_ref()` → `map()` → `unwrap_or_else()`.
- Why all match arms must return the same type.

## Files to touch
- `src/app.rs`

---

## Step 1: Update `render_status_bar`

Replace the existing `render_status_bar` in `src/app.rs` with:

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

Note the em-dash in `"Ready — {session_id}"` and the `…` in `"Thinking…"`.

Two things changed compared with the Phase 02 version:

**The `Error` arm changed** from `e.as_ref()` to `format!("Error: {e}")`.
Before, the match arms returned borrowed `&str` and the caller did
`.to_string()`. Now all three arms return an owned `String`, so the trailing
`.to_string()` on `label` disappears. A match must return **one** type across
all arms — one owned arm forces the rest.

**`.as_ref().map(...)` — the `Option` ladder.**

```rust
self.session                  // Option<Session>
    .as_ref()                 // Option<&Session> — borrow instead of move
    .map(|s| s.id.clone())    // Option<String> — transform if present
    .unwrap_or_else(|| "not connected".to_string())  // String — fallback if None
```

> **Rust concept:** `as_ref()` on `Option`
> Turns `Option<Session>` into `Option<&Session>` so `map` does not move the session out of `self` — which would be an error, since this method only has `&self`. TypeScript equivalent: `session?.id ?? "not connected"`.

> **Rust concept:** `.unwrap_or_else(|| ...)`
> Like `.unwrap_or(...)` except the fallback is a function, so the default string is only built when it is actually needed.

> **Rust concept:** `Status::Error(ref e)`
> `ref` binds by reference inside the pattern instead of moving the `String` out of the variant. Same pattern as Phase 01.

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

If the status bar shows an error containing `No HttpClient available`,
Phase 03b Step 1 was skipped — `.with_http_client(...)` is missing from
`main.rs`. The full message reads
`connect: GET /global/health: No HttpClient available`.

If it says `invalid URL` or a connection error, check that `opencode serve` is
running on `http://127.0.0.1:4096`.

---

Once the status bar shows a session ID, **Phase 03 is complete**. Move to
**Phase 04: Send Prompts**.
