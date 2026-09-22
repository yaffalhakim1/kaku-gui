# Phase 05a — Stream plumbing: event enum, channel, drain loop

> This phase was split from `05-sse-streaming.md` (2026-09-22) for pacing.
> This is **part 1 of 3**. Prerequisite: Phase 04 is complete.
>
> **Status: not started.**

## What you will build

The plumbing that lets a background task talk to the UI: a `StreamEvent`
enum, a channel field on `KakuApp`, and a drain loop in `render`. Nothing
connects to the network in this part — the reader task arrives in 05b. The
app compiles and runs exactly as before; this is infrastructure only.

## Concepts you will learn
- Enum variants carrying data (tuple vs struct variants).
- `std::sync::mpsc` channels — many senders, one receiver.
- Draining a channel inside `render` without fighting the borrow checker.

## Files to touch
- `src/app.rs`

---

## Step 1: Add a `StreamEvent` enum

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

## Step 2: Add an event channel to `KakuApp`

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
> The receiving half of a channel. `mpsc` means "multi-producer, single-consumer": many senders, one receiver. The background task (05b) holds the `Sender`, the UI holds the `Receiver`. This is the standard-library channel, so it adds no dependency.

---

## Step 3: Drain events in `render`

Update `render` to drain the channel at the top:

```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    while let Some(event) = self.events.as_ref().and_then(|rx| rx.try_recv().ok()) {
        self.apply_event(event, cx);
    }

    ...
}
```

Add `apply_event` in its own `impl` block:

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

`message.text = text` **assigns** rather than appends, because the event
carries the full text of the part so far (not an increment — that fact is
explained in 05b). Appending would duplicate everything on every frame.

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

```bash
cargo check
```

**Expect 0 errors.** `apply_event` is never called with real events yet (the
channel is always `None`), so some arms will be dead code — warnings about
that are expected and harmless at this point.

`cargo run` behaves exactly as it did at the end of Phase 04. Nothing visible
changed. The plumbing is in place; the water arrives in 05b.

---

Once this compiles clean, move to **Phase 05b: Read the stream**.
