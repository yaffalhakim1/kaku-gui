# Kaku GUI: Learning Log

Notes from building a GPUI desktop client for opencode, one phase at a time.

Verified against the `egoist/zed` `waku-webview` fork, gpui 0.2.2, September 2026.

Append one section per phase. Newest phase at the bottom. The glossary grows; nothing else gets rewritten.

---

## How this log works

Each phase gets three parts:

- **What I built**: the concrete result
- **Concepts that landed**: the ideas I understand now
- **Misconceptions**: things I got wrong and what corrected them

The misconceptions are the useful part. Reading someone's finished understanding teaches you less than reading where they went sideways.

---

## Rust foundations

These came up in Phase 00 and Phase 01, and they keep coming back. Learn these once and the rest of the project goes faster.

### Ownership

Rust gives every value exactly one owner. Assigning a value moves it.

```rust
let a = String::from("kaku");
let b = a;
println!("{}", a);   // error: a was moved
```

JavaScript hands out aliases. Two variables, one object, and the garbage collector keeps it alive while anything points at it. Rust has no collector, so one owner decides when memory frees. That constraint is the source of everything else in this section.

### Borrowing

`&` means "borrow, do not take."

```rust
let b = a;    // move: a is dead
let b = &a;   // borrow: a is still usable
```

Function signatures spell the choice out:

```rust
fn render_messages(&self, messages: Vec<DisplayMessage>)    // takes ownership
fn render_messages(&self, messages: &Vec<DisplayMessage>)   // borrows
```

Almost every GPUI parameter carries a `&`. Nothing takes ownership of the app context; it gets borrowed for the duration of a call and handed back.

### self

`self` is the struct a method belongs to. JavaScript's `this`.

```rust
impl KakuApp {
    fn render_messages(&self, ...) { ... }
}
```

Inside that method, `self.theme` is this app's theme and `self.messages` is this app's messages.

| Form | Meaning |
|---|---|
| `&self` | borrow, read only |
| `&mut self` | borrow, allowed to change |
| `self` | consume |

### Copy vs Clone

`Clone` duplicates a value when you call `.clone()`. `Copy` duplicates it silently on assignment.

| | `Copy` | `Clone` |
|---|---|---|
| Cost | trivial | possibly expensive |
| How you duplicate | plain assignment | explicit `.clone()` |
| Examples | `i32`, `bool`, `Theme`, `Hsla` | `String`, `Vec<T>`, `FocusHandle` |

A type earns `Copy` from its definition, not from how often you use it. All of `Theme`'s fields are small flat numbers, so `Theme` qualifies. `FocusHandle` points at something GPUI owns, so duplicating it creates a second handle to one object, which is a deliberate act worth writing down.

That distinction explains the two clone triggers:

| Trigger | Example | Why |
|---|---|---|
| Returning owned data from `&self` | `fn focus_handle(&self) -> FocusHandle` | cannot move a field out of a shared borrow |
| Passing a field onward from `&self` | `div().bg(self.theme.background)` | same rule, avoided only by `Copy` |

The second row is why `Theme` needs `#[derive(Copy)]` to make `self.theme.text` work at all. Strip `Copy` and every `.bg(self.theme.background)` needs an explicit `.clone()`.

### Entity<T> is a handle

`Entity<KakuApp>` is not `KakuApp`. It is a reference-counted pointer to a `KakuApp` that GPUI owns and tracks.

```rust
pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
    cx.new(|cx| Self { ... })
}
```

`cx.new` allocates the component on GPUI's heap, wraps it in a handle, and returns that handle. A plain `KakuApp` value in a local variable has no address, so GPUI would have nothing to call `render()` on. The handle is what makes the component findable and shareable.

React parallel: `Entity<T>` is the mounted instance, not the component function. A `useRef` gives you the live mounted instance even though React owns it.

`FocusHandle`, `Entity<T>`, and `Arc<T>` are all the same pattern. Reference-counted handles exist so multiple parts of an app can reference one thing without moving it.

### IntoElement

`IntoElement` is a trait: "GPUI can render this." Think of TypeScript's `ReactNode` union.

`render()` returns `impl IntoElement` rather than a named type because GPUI's element types are nested generics like `Div<Stateful<Div>>`. `impl` means "returns something satisfying this trait, and the concrete type does not need a name."

### Lazy return values

`-> impl IntoElement` means the function returns a *description* that captures values, not a finished element. GPUI materializes it later. Anything the returned value captures has to outlive the call, which is why helper methods take owned data and why closures returning elements carry `move`.

---

## Phase 00: Scaffold

**Result:** three files, one dark window, centered "Hello Kaku".

| File | Role |
|---|---|
| `src/main.rs` | opens the window, like `index.tsx` |
| `src/app.rs` | the root component, holds state |
| `src/theme.rs` | color palette, like CSS custom properties |

### React mapping

| GPUI | React |
|---|---|
| `struct KakuApp` | component state shape |
| `impl Render for KakuApp` | the component's function |
| `Entity<KakuApp>` | mounted instance |
| `Context<Self>` | context plus `setState` |
| `div().flex()` | flexbox |
| `.child(x)` | nesting a child element |

### The three core types

`App` is the global application context. `Entity<T>` is a handle to one component instance. `Context<Self>` is the per-component context that lets a component update itself.

`impl Render for KakuApp` is the piece that clicked. GPUI calls `render()` when it needs the UI, gets an element tree back, discards it, and calls again when state changes. React does the same thing with component functions.

### Notes

`mod app;` declares a module. Rust will not find `src/app.rs` without it.

`Theme` derives `Clone, Copy, Debug`. `Copy` is what lets `render` read `self.theme.background` several times without cloning.

`px(24.0)` takes a length type. There is no `"24px"` string.

---

## Phase 01: Static Chat Layout

**Result:** message list, input bar, status bar. All data hardcoded.

### Layout regions

```
<div flex-col height 100%>          render()
  ├─ <div messages>  flex_1        render_messages()
  ├─ <div input>     height 48     render_input_bar()
  └─ <div status>    height 24     render_status_bar()
</div>
```

`.flex_1()` on the message list means "take the remainder." It fills the window because the other two children claimed fixed heights. Delete `.flex_1()` and the list collapses to its content height, leaving a large empty gap below the status bar. The bars appear to jump to the top of the window.

That is the diagnostic: bars floating in the middle means a missing `.flex_1()` somewhere.

Helper methods return `impl IntoElement` and take an owned `Vec`, a `Theme`, and `&self`. Each one builds one region of the tree. Same shape as composing React sub-components, except these are methods on one struct rather than separate types.

### match

`match` must handle every variant. Leave one out and the build fails, pointing at the arm you missed.

```rust
let (prefix, color) = match m.role {
    Role::User => ("› ", theme.user),
    Role::Assistant => ("", theme.text),
    Role::System => ("", theme.muted),
};
```

That exhaustiveness is why adding `Role::Tool` later will list every site needing an update. TypeScript's `switch` lets you forget an arm and hands you `undefined` at runtime.

### enum variants can carry data

```rust
pub enum Status {
    Idle,
    Busy,
    Error(String),
}
```

`Error(String)` is one value holding both the state and the message. TypeScript would model this as `{status: 'error', message?: string}`, where `message` can be missing exactly when you need it. Rust makes the message mandatory in that variant and unreadable outside it.

### Ownership pressure in render

```rust
fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    let theme = self.theme;
    let messages = self.messages.clone();
    let status = self.status.clone();
    ...
}
```

Three locals exist because of one rule: you cannot borrow `self` for a method call and borrow `self` again for that call's argument inside one expression.

`theme` copies for free. `messages` and `status` need `.clone()` because `Vec` and `Status` are not `Copy`.

`#[derive(Clone)]` on `DisplayMessage` is load-bearing here. `Vec<T>: Clone` requires `T: Clone`. Remove the derive and the line stops compiling.

The clone is wasteful. It duplicates the transcript every frame. Phase 06 fixes it. Phase 01 chooses the version that does not need lifetimes.

### into_iter

Three ways to loop a `Vec`, and they differ by ownership:

| Form | Borrows or consumes |
|---|---|
| `&messages` | borrows each item |
| `&mut messages` | mutably borrows each |
| `messages.into_iter()` | consumes the list, yields owned items |

`into_iter()` yields owned items, which is what `render_message(m: DisplayMessage)` wants.

### ref

`Status::Error(ref e)` avoids moving the `String` out of the enum. Since `status` is owned here, a plain `e` would take the message and leave `status` unusable. `ref` says "lend me a reference instead." This pattern shows up whenever you match on an owned enum whose variants hold data.

### Deferred question

`render_messages` takes `Vec<DisplayMessage>` by value rather than `&Vec<DisplayMessage>`. The borrow version fails because `-> impl IntoElement` returns a value that captures its inputs, and a reference passed in from a temporary cannot outlive the return. The borrow version compiles with an explicit lifetime parameter, at the cost of threading a lifetime through the struct. Revisit in Phase 06.

---

## Phase 02: Text Input

**Result:** the placeholder input bar is a real text field. Typing works, Enter submits, the input clears.

`src/input.rs` introduces `TextInput` as its own component:

```rust
pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
}
```

### A child component is just another Entity

`KakuApp` does not own a `TextInput`. It owns an `Entity<TextInput>`:

```rust
input: Entity<TextInput>,
```

`cx.new(|cx| TextInput::new(cx))` allocates the child on GPUI's heap and hands back a handle. The parent holds the handle, not the value. React analogy: the parent holds a ref to a child instance, not a copy of the child's state.

That matters because the parent needs to read the text at submit time and clear it afterward. Holding a handle makes both possible without moving anything.

### `FocusHandle` and `track_focus`

`FocusHandle` is a handle to "who currently has keyboard focus." `TextInput` creates one with `cx.focus_handle()` and claims it in render:

```rust
.track_focus(&self.focus_handle)
```

`track_focus` is what routes keystrokes to this element. Without it, `on_key_down` never fires and the app looks broken while compiling fine. This is the first of several GPUI problems that produce no compiler error.

The window also has to give focus to something at startup, which is why `main.rs` calls `window.focus(&focus, cx)` with the root component's handle.

### Keystrokes arrive as `KeyDownEvent`

```rust
.on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
    if let Some(c) = event.keystroke.key_char.as_ref() {
        if c.len() == 1 && !event.keystroke.modifiers.shift {
            this.content = format!("{}{}", this.content, c).into();
            cx.notify();
        }
    }
    if event.keystroke.key == "backspace" {
        let mut s = this.content.to_string();
        s.pop();
        this.content = s.into();
        cx.notify();
    }
}))
```

Two things worth naming. `cx.listener(...)` is what makes `this` available — the closure receives the component itself, like an event handler bound to a component instance rather than a free function. And every mutation ends with `cx.notify()`, which is the `setState` equivalent: without it the state changes and the UI never redraws.

`Backspace` has no `key_char`, so it is handled by `key` instead.

The `!event.keystroke.modifiers.shift` guard is worth understanding before trusting it. On Windows, `key_char` is produced by `ToUnicode` using the live keyboard state, so Shift+A arrives as `key_char = Some("S")` with `modifiers.shift == true`. The guard therefore *rejects* shifted characters — **capital letters cannot be typed yet.** That is a known limitation of this phase, not a feature. Real text input needs `EntityInputHandler`, which is what GPUI's own `examples/input.rs` uses. Replacing this hand-rolled `on_key_down` handler is future work, not Phase 03.

### `SharedString` instead of `String`

`content` is a `SharedString`, not a `String`. It is a cheaply-cloneable, refcounted string — GPUI wants text that can be handed to the renderer without copying. `format!(...).into()` converts a `String` into one, and `self.content.to_string()` goes the other way.

The `.into()` calls look like noise until you notice the field type is not `String`. Every assignment has to produce a `SharedString`.

### Actions: `SendPrompt`

Enter is not wired directly to a callback. It goes through an action:

```rust
actions!(kaku_gui, [SendPrompt]);
```

then in `main.rs`:

```rust
cx.bind_keys([KeyBinding::new("enter", SendPrompt, Some(("KakuApp")))]);
```

and in `render`:

```rust
div()
    .key_context("KakuApp")
    .on_action(cx.listener(Self::send_prompt_action))
```

Three pieces have to line up: the key binding, the `key_context` on the focused subtree, and an `on_action` handler inside that context. Drop any one and Enter silently does nothing.

The reason to use an action rather than `on_key_down` here is separation: the keymap decides *which* keys trigger submission, and the handler only knows *that* submission happened. Rebinding to `ctrl+enter` later is a one-line change in `main.rs` and touches no UI code. This is the same reason React apps route shortcuts through a command layer instead of scattering `keydown` listeners.

The handler reads the child's state through its handle, then pushes to its own `messages`:

```rust
let content = self.input.read(cx).content().clone();
```

`self.input.read(cx)` borrows the child component immutably. `.clone()` is needed because the borrow ends when the expression does, and `content()` returns a reference into the child.

### Why `main.rs` also changed

`mod input;` registers the new module, and the window now focuses `KakuApp` instead of nothing. Phase 02 touched `main.rs` and `app.rs` as well as adding `input.rs`.

---

## Misconceptions

### "`Copy` is earned by using a type a lot"

`Copy` comes from the type definition. `Theme` gets it because all seven fields are flat numbers with no heap allocation. Usage does not grant it.

### "Rust forbids two things from being used together"

The rule is one owner at a time. Three ways to work with it:

| | Original still usable | Lifetime concerns |
|---|---|---|
| move | no | none |
| `&` borrow | yes | must not outlive the owner |
| handle (`.clone()`) | yes | none |

Passing `TextInput` to a child by value would move it out of `KakuApp`, and the parent needs it later to read what the user typed. That is why handles get cloned instead.

### "Cloning is something to avoid"

Cloning small `Copy` values costs nothing. Cloning handles like `Entity<T>` and `FocusHandle` copies a pointer. Only cloning heap data like `String` and `Vec` duplicates real memory, and Rust makes those cases visible by requiring the explicit `.clone()`.

### "`&` means reference, like in JS"

Rust references behave differently. The compiler tracks how long each one lives and rejects any that outlive their data. A reference that would dangle is a build failure, not a runtime surprise.

---

## Glossary

| Term | Meaning |
|---|---|
| `App` | global application context |
| `Entity<T>` | handle to a GPUI-owned component instance |
| `Context<Self>` | per-component context, includes the ability to update |
| `Render` | trait a type implements to be drawable |
| `IntoElement` | trait meaning "GPUI can render this" |
| `FocusHandle` | handle to keyboard focus |
| `Theme` | the color palette struct |
| `DisplayMessage` | one chat message: role plus text |
| `Role` | enum: User, Assistant, System |
| `Status` | enum: Idle, Busy, Error(String) |
| `&self` | borrow the component, read only |
| `&mut self` | borrow the component, may change |
| `&` | borrow instead of take |
| move | transfer ownership, original dies |
| `.clone()` | explicit duplicate |
| `Copy` | silent duplicate on assignment |
| `match` | pattern match, must be exhaustive |
| `ref` | bind by reference inside a pattern |
| `.into_iter()` | consume a collection, yield owned items |
| `.children()` | render many elements at once |
| `px(v)` | a length value |
| `Action` | a named, rebindable command (`SendPrompt`) |
| `actions!(...)` | macro declaring a set of actions |
| `KeyBinding::new` | maps a keystroke to an action |
| `.key_context(...)` | names a subtree so keybindings can target it |
| `.on_action(...)` | handles an action dispatched in this context |
| `cx.listener(...)` | closure that receives the component as its first argument |
| `cx.notify()` | tell GPUI to re-render this component |
| `cx.new(...)` | allocate a child component, get an `Entity<T>` back |
| `.read(cx)` | borrow a child component immutably |
| `.update(cx, ...)` | borrow a child component mutably |
| `track_focus` | route keyboard input to this element |
| `SharedString` | cheaply-cloneable refcounted text |
| `KeyDownEvent` | a key press delivered to a focused element |
| `key_char` | the character a keystroke would type, if any |
| `Arc<T>` | reference-counted shared pointer, thread-safe |
| `dyn Trait` | "some type implementing this trait, chosen at runtime" |
| trait | interface; defines required methods without implementations |
| dependency injection | constructor receives its dependencies instead of creating them |
| `::` vs `.` | `Type::f()` needs no instance; `x.f()` operates on one |
| `Result<T>` | value that is `Ok(T)` or `Err(error)`; errors are values, not throws |
| `?` | "if Err, return it now; if Ok, unwrap and continue" |
| `.context(...)` | attach a human-readable message to an error (anyhow) |
| `bail!` | return `Err(...)` from the function right now |
| `async fn` | function returning a future; does nothing until polled |
| `.await` | pause here until the future completes |
| future | value representing work not yet done |
| turbofish `::<_>` | explicitly name types the compiler cannot infer |

---

## Open questions

- How to avoid the per-frame `messages.clone()` without adding lifetimes to `KakuApp`. Phase 06b territory.
- Whether `.flex_1()` is the right tool for the transcript once the list scrolls. Phase 06b adds scrolling.

---

## Phase 03a: Client Types and HTTP Methods

**Result:** `src/client/` with `types.rs` and `mod.rs` — `Health`, `Session`, `SessionTime` structs, and `OpencodeClient` with `health()` and `create_session()`. Compiles with 0 errors, 9 warnings (all expected: nothing constructs the client yet).

Typed and verified 2026-09-22, against OpenCode 1.18.31 live (`/global/health` and `POST /session` response shapes confirmed with real requests).

### Dependencies: the corrected story

The AGENTS.md claim that all seven crates were "already in `Cargo.lock` via gpui" was **wrong for one of them**. Measured truth:

- `anyhow`, `serde`, `serde_json`, `futures` — already in the lock via `gpui`. Adding them as direct dependencies is free: it grants naming permission, compiles nothing new.
- `reqwest_client` — **not** in the lock. It brought **78 new crates** (tokio, zed-reqwest, hyper, h2, rustls, tower) and the first `cargo check` took ~1.5 min.

The lock went from 707 to 787 packages. `reqwest_client` is still the right choice — it is the only working `HttpClient` impl in the pinned fork — but the "free" claim cost an afternoon of confusion in an earlier session.

### The three new structs

`Health` derives `Deserialize` only (it only ever arrives from the server). `Session` derives both `Serialize` and `Deserialize` (it will be sent later too). `#[serde(rename = "projectID")]` maps the camelCase JSON key onto the snake_case Rust field without renaming the field.

Serde **ignores** JSON fields the struct does not declare — the real `/session` response also carries `slug`, `cost`, `tokens`, and `path`, and none of those need to exist in the struct. Parsing what you need is not a bug.

### `Arc<dyn HttpClient>` — read it inside-out

- `HttpClient` is a **trait** — an interface. GPUI defines it; `ReqwestClient` is one implementation.
- `dyn` means "some type implementing this trait, decided at runtime." Explicit in Rust; implicit in TypeScript because interfaces are structural.
- `Arc` is a reference-counted pointer: several owners share one value. Thread-safe sibling of `Rc`.

So the field reads: "a shared, runtime-chosen HTTP client." Same *idea* as `Entity<T>` — a handle to something you do not own directly.

### The client receives its HTTP client

`OpencodeClient::new(base, http)` takes the `Arc<dyn HttpClient>` as a parameter — dependency injection. Two consequences:

1. `new` **cannot fail**, so it returns `Self`, not `Result<Self>`.
2. `new` takes **no `self` parameter** — it creates the value, so there is nothing to borrow yet. That is why it is called with `::` (`OpencodeClient::new(...)`), like a static method. Same pattern as `Theme::dark()`.

Rule that fell out of the discussion: *if you can't write `self.something` in the body, the function shouldn't take `self`.*

### Error handling shape

`Result<T>` with `?` propagation, `anyhow::Context` layering the error chain, `bail!` for early `Err` returns. The contexts stack: `join ... onto ...` → `GET /global/health` → `read health body` → `parse health JSON` — so when something fails, the message names the exact step.

The request body is a **private struct declared inside the function** (`NewSession { title }`), not a `json!` macro call. A typo'd field name is a compile error instead of a runtime 400. Same reasoning as preferring typed API responses over `any` in TS.

### `async`/`await` — first contact

`async fn` produces a *future* — a value representing work not yet done. `.await` pauses until it finishes. Nearly identical syntax to JS; the difference is that a Rust future does **nothing** until something polls it (JS promises run on an event loop by default). Async is contagious: a function containing `.await` must itself be `async`.

The actual awaits live in the methods (`health`, `create_session`); something else (Phase 03b's `connect`) must drive them.

### The borrow discussion (session 2 of Phase 03)

Three questions from my own code, two wrong answers, corrected in chat:

| Question | My answer | Verdict |
|---|---|---|
| Why `&mut self` on `clear`? | "it borrows content and changes it" | ✅ correct — but it borrows the *whole instance*, not just the field |
| Why no `self` on `new`? | "nothing calls it yet" | ⚠️ right conclusion, wrong reason — `self` is a *parameter*, and `new` creates the value so there is nothing to borrow |
| Why `.clone()` on `content().clone()`? | "I'll also borrow it" | ⚠️ right instinct (there IS a conflict), wrong mechanism — `.clone()` **ends** the borrow by copying |

The E0502 error, reproduced in a scratch crate to see it for real:

```
error[E0502]: cannot borrow `self.child` as mutable because it is also borrowed as immutable
```

`content()` returns a reference *into* the child's memory; `clear()` needs `&mut` on the same memory; holding both is a use-after-free — the class of bug Rust exists to prevent. `.clone()` is not "I'll also borrow", it is **"I'll take a copy so I no longer need the borrow."**

Rules that landed:

1. `self` is a parameter, not a keyword. No `self` → call as `Type::f()`.
2. `&self` reads, `&mut self` changes, `self` consumes. Compiler-enforced.
3. Many `&` OR one `&mut`, never both. `.clone()` escapes the conflict by producing an owned value.

### Project pacing decision

Phase 03's original single file (365 lines, ~7 new concepts) was too much for one sitting. **The phase was split into 03a/03b/03c**, and the splitting convention was added to AGENTS.md: split any phase that would introduce more than ~4 new Rust concepts per sitting; parts must compile standalone; each carries a status header; `task.md` records the position. Phases 05 and 06 were split the same way in the same session.

**Misconception logged:** "the project itself is too complex and should be scaled down." Examined and rejected — the wall was pacing, not scope. The difficulty map showed Phase 03 and 05 as the two peaks; 04/07/08 are gentle. A smaller project (todo app, CLI) hits the same Arc/async/Option wall with none of the accumulated momentum. Verdict: continue, one part per sitting.
