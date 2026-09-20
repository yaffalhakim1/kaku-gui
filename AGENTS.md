# Kaku GUI — Session Context

## Learner profile

- Background: experienced web developer (frontend + backend).
- Rust knowledge: 0.
- Desktop / GPU UI knowledge: 0.
- Learning style: learn by doing. Wants to type the code themselves.
- Preferred pace: one small phase per session, like an 8-hour tutorial split into episodes.

## Project goal

Build a native GUI client for OpenCode using Rust + GPUI (https://gpui-kit.com/) (same stack as waku `C:\Users\yafit\Documents\Learn\rust\waku`).
This is a personal learning project, not production code.

## Workflow (MANDATORY)

1. The project is split into phases under `docs/phases/`.
2. Each session, the user picks ONE phase by saying something like:
   - "read phase 01"
   - "let's do phase 03"
   - "explain phase 00"
3. The assistant MUST:
   - Read the requested phase markdown.
   - Explain the phase goal and Rust / GPUI concepts.
   - Provide code blocks for the user to type themselves.
   - NOT write the code into files unless explicitly asked.
4. The user does the heavy lifting: typing, compiling, and debugging.
5. The assistant acts like a tutorial narrator + debugger.

## Phase roadmap

**This section is the single source of truth for the roadmap.** If `docs/implementation_plan.md` or `task.md` disagrees with it, this section wins — those files are historical notes.

- Phase 00: Recap the minimal scaffold (3 files: main.rs, app.rs, theme.rs).
- Phase 01: Static chat layout.
- Phase 02: Text input + submit on Enter.
- Phase 03: Connect to OpenCode (health + session).
- Phase 04: Send prompts.
- Phase 05: SSE streaming.
- Phase 06: Abort, scroll, status polish.
- Phase 07: Slash commands (/clear, /model, /quit).
- Phase 08: Waku-inspired UI/UX polish (`docs/phases/08-waku-uiux.md`).
- Markdown / reasoning rendering is OUT OF SCOPE until Phase 07 is done. It is not Phase 08.

The phase files that exist today are `00-scaffold.md` through `08-waku-uiux.md`. Phase 00 is a _recap_ of a scaffold that already exists in `src/`; do not re-create it.

**Progress through the roadmap lives in `task.md`.** This file does not track
which phases are done or which source files exist; check `task.md` for both so
the two cannot drift apart.

## Stack facts (verified — do not guess)

- **GPUI comes from a fork.** `gpui` and `gpui_platform` are both
  `git = "https://github.com/egoist/zed", branch = "waku-webview"`.
  These are **not** crates.io dependencies. gpui reports version `0.2.2`.
- The fork is Zed upstream `main` plus PR #61945 (layered scene rendering), which
  lets GPUI composite menus and tooltips above native child views. Reasoning:
  drop back to upstream once that PR merges.
- `gpui_platform` is required for `application()`. It is a separate crate from
  `gpui`; you cannot open a window with `gpui` alone.
- kaku-gui is pinned to the same fork as waku. kaku-gui is **Windows-first**,
  so the fork's macOS WebView concerns are irrelevant here.
- Toolchain: stable Rust (MSVC target on Windows). If you see an error about
  `cold_path`, the toolchain is too old — `rustup update stable`.
- `Cargo.toml` profiles are deliberate: `[profile.dev] opt-level = 1` and
  `[profile.dev.package."*"] opt-level = 2` keep GPUI's text shaping and layout
  hot paths from running fully unoptimized in debug builds.
- **Dependencies are fixed to an approved set.** No new dependency without
  asking first. The approved set is:

  | Crate | Why |
  |---|---|
  | `gpui` | the UI framework |
  | `gpui_platform` | `application()` lives here, not in `gpui` |
  | `anyhow` | error propagation |
  | `reqwest_client` | the working `HttpClient` impl (same fork/branch) |
  | `serde` (derive) | `#[derive(Deserialize)]` |
  | `serde_json` | JSON parsing |
  | `futures` | `AsyncReadExt` / `AsyncBufReadExt` to read response bodies |

  All seven are already in `Cargo.lock` via `gpui`, so adding them introduces
  no new transitive tree. Anything outside this set still needs to be asked for.

## GPUI API rules (MANDATORY)

- **Never write a GPUI API from memory.** GPUI has almost no public docs and
  churns constantly. Every GPUI symbol you quote must come from the pinned
  checkout on disk.
- The pinned checkout is at:
  `C:\Users\yafit\.cargo\git\checkouts\zed-4d64e9894aeee3ad\57bd4fe`
  → `crates/gpui/`, `crates/gpui_platform/`, `crates/http_client/`,
  `crates/reqwest_client/`.
  That rev (`57bd4fe181639797d395978d5de17bc9e10a6219`) is what kaku-gui's
  `Cargo.lock` resolves to. **`f9bad89` is waku's rev, not ours** — both
  checkouts sit side by side on disk, so pointing at the wrong one fails
  silently. Confirm the rev against `Cargo.lock` before reading.
- The best examples are inside that checkout's `crates/gpui/examples/`: read
  `input.rs` (text input, `EntityInputHandler`), `list_example.rs` and
  `uniform_list.rs` (virtualized lists), `scrollable.rs` (scroll handles),
  `animation.rs`, `popover.rs`.
- `https://gpui-kit.com/` is a **reading reference only**. It is not a
  dependency and its snippets may target a different GPUI revision. Verify
  anything you take from it against the pinned checkout before teaching it.
- Zed's own `ui`, `theme`, and `component` crates exist in the same fork, but
  kaku-gui does **not** use them. All UI is hand-rolled with `div()`. Do not
  propose adding them.
- Useful confirmed API facts (still verify before quoting):
  - `div().id("...")` returns `Stateful<E>`; `.hover()`, `.active()`, `.focus()`,
    `.on_click()`, `.on_hover()` live on `StatefulInteractiveElement` /
    `InteractiveElement`, reachable via `gpui::prelude::*`.
  - `cx.background_executor()` and `cx.spawn()` on `App` / `Context<T>`.
- **async in GPUI is smol, not tokio.** Concretely:
  - Use `cx.spawn(...)`, `cx.background_executor().spawn(...)`, and `Task<T>`.
    `Task` is cancelled when dropped; call `.detach()` to let it run.
  - **The async-closure form is the one that compiles:**
    `cx.spawn(async move |this, cx| { ... })`. The two-argument form
    `cx.spawn(|this, mut cx| async move { ... })` fails with
    `E0282: type annotations needed`. Do not teach the second form.
  - Results come back into the UI through a channel that `render` drains — the
    UI must never block on a future.
  - Do not add a tokio runtime to the app. `ReqwestClient` owns one internally;
    the app never constructs one and never calls into it directly.
  - Do not _teach_ any of this before its phase. It is recorded here so the
    assistant plans correct phases, not so the user learns it early.

- **HTTP: `cx.http_client()` is not enough on its own.** This is the trap that
  wastes an afternoon:
  - `gpui_platform::application()` calls `Application::with_platform`, which
    installs `NullHttpClient`. Every request through it fails at runtime with
    `No HttpClient available` — there is **no compile error**.
  - The app must install a real client at startup:
    `application().with_http_client(Arc::new(ReqwestClient::new())).run(...)`.
  - After that, `cx.http_client()` (on `App`, and on `Context<T>` via `Deref`)
    returns the working client.
  - Request/response types come from `gpui::http_client`:
    `AsyncBody`, `HttpClient`, `Method`, `Request`, `Url`, `Json`.
    `get(uri, body, follow_redirects)` and `post_json(uri, body)` return
    `Response<AsyncBody>`.
  - Read bodies with `futures::AsyncReadExt::read_to_string`, or stream lines
    with `futures::AsyncBufReadExt` over `futures::io::BufReader`.

- **The re-export trap.** `gpui` re-exports `serde`, `serde_json`, `anyhow`,
  `http_client`, and `Url`, which tempts you into thinking no dependency is
  needed. Two things still need a **direct** dependency:
  - `#[derive(Deserialize)]` fails with `E0463: can't find crate for serde`
    unless `serde` is a direct dependency. The re-export does not satisfy the
    derive macro.
  - `futures` is not re-exported by `gpui` at all, and reading a body needs
    `AsyncReadExt` from it.

## Teaching style

- Keep explanations short and concrete.
- Include "Rust concept" callouts inside each phase (e.g., what `&mut self` means).
- Prefer web-dev analogies:
  - GPUI `Entity<T>` ≈ React component instance
  - `Render` trait ≈ React `render()` method
  - `Context<T>` ≈ React context + setState combined
  - `div().flex()` ≈ CSS flexbox
- Never assume the user knows Rust syntax, ownership, lifetimes, or traits.
- Define every Rust word the first time it appears
- Show the JS equivalent next to it when one exists
- Assume you’ve never seen the syntax, so spell out what each symbol does
- Say “this one is new” when a concept hasn’t come up yet

## Code style

- Modules are added phase by phase as the user types them. See `task.md` for
  which files exist right now.
- Follow the existing kaku dark theme palette.
- Keep the scaffold compiling after every phase.

## Important constraints

- Do not jump ahead to future phases.
- Do not introduce concepts before their phase.
- Do not auto-fix compile errors for the user unless asked; guide them to the fix instead.
- When the user says "read phase XX", read `docs/phases/XX-*.md` and teach from it.

## Tutorial mode vs. the artifact rules

In a tutorial session the assistant writes **no code into `src/`**. The master
rules' artifacts (`implementation_plan.md`, `task.md`, `walkthrough.md`) are
narrated in chat during tutorial sessions, not written to disk, unless the user
explicitly asks for a file.

## Reference material (read-only, outside this workspace)

These live outside the kaku-gui workspace root, so they may not be readable
without asking the user first. Do not assume access.

| Path                                           | What it is for                                                                                                  |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `C:\Users\yafit\Documents\Learn\rust\waku`     | Same GPUI stack. Design reference for Phase 08, and `src/ui/` shows hand-rolled widgets in this exact fork.     |

Also worth knowing: waku's `Cargo.toml` documents _why_ the fork exists (see the
comment above its `gpui` dependency), which is the authority if you ever doubt
the fork-vs-upstream decision.
