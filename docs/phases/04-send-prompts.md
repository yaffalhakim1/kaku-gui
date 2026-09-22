# Phase 04 — Codex protocol (pointer stub)

> **Revised twice on 2026-09-22.** The first Codex draft (single file) had
> two design bugs caught on review, so it is split and corrected:
>
> 1. The original created a fresh `BufReader` per read — buffered-but-unread
>    lines were silently dropped.
> 2. It stored the client in the UI *and* moved it into a task. The blocking
>    stdout reader and the request sender need separate ownership.
>
> The corrected architecture: the UI owns a cloneable **stdin handle** for
> sending requests; a background thread exclusively owns the **stdout
> reader** and forwards everything as events.

Read and work through the parts in order:

| Part | File | Scope |
|---|---|---|
| 04a | [`04a-codex-client.md`](04a-codex-client.md) | types, spawn, initialize handshake, request handle |
| 04b | [`04b-connect-wiring.md`](04b-connect-wiring.md) | reader thread, event channel, drain loop, thread/start |

Each part compiles on its own with 0 errors.
