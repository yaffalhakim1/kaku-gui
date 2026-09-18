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

---

## Open questions

- How to avoid the per-frame `messages.clone()` without adding lifetimes to `KakuApp`. Phase 06 territory.
- Whether `.flex_1()` is the right tool for the transcript once the list scrolls. Phase 06 adds scrolling.

