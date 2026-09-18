# kaku-gui

Native GUI client for [opencode](https://opencode.ai), built with Rust + GPUI (the same stack as [waku](https://github.com/egoist/waku)).

This is a personal learning project. The scaffold is intentionally minimal: just a dark GPUI window that says "Hello Kaku". Every feature is added phase by phase by typing the code yourself.

## Run

```bash
cargo run
```

## Project layout

```
src/
  main.rs   # Opens the application window
  app.rs    # The root UI component (like a React root component)
  theme.rs  # Color palette
docs/
  phases/   # One markdown lesson per phase
```

## Learning workflow

1. Open one phase markdown from `docs/phases/`.
2. Read the explanation and type the code blocks yourself.
3. Run `cargo check` or `cargo run` after each step.
4. Only move to the next phase when the current one compiles.

See `AGENTS.md` for the full session context.

## License

MIT
