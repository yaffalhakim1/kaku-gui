# Phase 05 — SSE Streaming

> **This phase was split into three smaller files (2026-09-22) for pacing.**
> The original single 239-line file introduced 8 new concepts in one sitting.
>
> Read and work through them in order:
>
> | Part | File | Scope |
> |---|---|---|
> | 05a | [`05a-stream-plumbing.md`](05a-stream-plumbing.md) | `StreamEvent`, channel field, drain loop in `render` |
> | 05b | [`05b-read-stream.md`](05b-read-stream.md) | `event_stream()` client method, SSE reader task, parsing |
> | 05c | [`05c-stream-e2e.md`](05c-stream-e2e.md) | `send_prompt`, `streaming_idx`, end-to-end verify |
>
> Each part compiles on its own with 0 errors. Do not start a part before the
> previous one is clean.
