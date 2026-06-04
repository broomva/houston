# Agent cross-review (native second opinions)

Phase 8 of RFC #248, **reframed**. Not an `advanced.*` feature flag and not a runtime gate — it is **default agent doctrine + an engine capability**. Every Houston agent, by its own judgment, gets a second opinion on important work from a different AI model before presenting it as final. The productized form of bstack P20 cross-review, de-jargoned to fit the non-technical product voice.

Default: **on** (embedded doctrine, not opt-in). Status: **beta**. Caller in v1: anthropic only.

## What it does

When an agent finishes substantial, risky, or hard-to-undo work (a plan, analysis, recommendation, draft, or consequential change), it asks an independent reviewer — a *different* AI model — to adversarially check the work, weighs the feedback, fixes real problems, and only then calls it done. For trivial replies it skips this. The user never sees the machinery; at most a one-line "I double-checked this and corrected a couple of things."

## Two parts (both required)

### Part A — capability (engine)
`engine/houston-engine-core/src/sessions/cross_review.rs` exposes a free function:

```
pub async fn run_review(caller: Provider, work_summary, work_content, focus) -> CoreResult<Value>
```

- Wraps `sessions::provider_oneshot::run_provider_oneshot` — governed (in-engine CLI resolution), not an ungoverned shell-out.
- **Reviewer selection** (`select_reviewer`): iterate `houston_terminal_manager::provider::all()`, skip the caller and any provider with no one-shot CLI (`life`), cheap-pre-check each with `crate::provider::check_status` (auth, no session spawn), pick the first authenticated *different* provider (registry order → openai before gemini, the stronger cross-model reviewer for an anthropic caller). If none usable, fall back to a **same-provider fresh-context** review (`crossProvider: false`) — always produces a second opinion, never silently fails.
- **Models** (`default_reviewer_model`): capable tier per provider — `sonnet` / `gpt-5.5` / `gemini-3.1-flash-lite` (CLI aliases; keep in sync with `app/src/lib/providers.ts`, NOT the stale `agent-manifest.md` table). Gemini stays flash-lite: the pro tier is gated behind paid Google plans (free-tier OAuth hangs).
- **Errors** surface as `CoreError::Internal` (user-initiated work → toast; no silent fallback, unlike `summarize`).

Exposed to the agent as the in-engine MCP tool `request_review` (`engine/houston-engine-server/src/routes/mcp.rs`): registered in `tools/list`, dispatched in `resolve_tools_call` → `resolve_request_review`. It rides the existing SSE keepalive path (a one-shot review can exceed claude's 60s first-byte budget). MCP is wired only for anthropic sessions today (`sessions/mod.rs` gates `mcp_config` on the anthropic provider), so the caller is anthropic; when MCP extends to other providers, resolve the caller from session context so the reviewer still differs.

### Part B — doctrine (product prompt)
`app/src-tauri/src/houston_prompt/cross_review.rs` (`CROSS_REVIEW_GUIDANCE`), wired into `system_prompt()` in `houston_prompt/mod.rs`. The behavioral half: makes the agent reach for the capability *generatively* (by judgment, not a hard rule). Plain product voice — no "P20", "cross-model", or tool/model names — and worded to **degrade gracefully** for providers without the tool ("if a review tool is available, use it; otherwise re-examine the work yourself with fresh, critical eyes"). The global product prompt is the only layer that reaches every agent uniformly, survives user CLAUDE.md edits, and applies to existing + new agents.

## Visibility

The `request_review` tool call and its result ride the NDJSON parser rails into the session feed automatically (every non-`AskUserQuestion` tool passes through unchanged), so v1 needs no explicit `FeedItem`/DB emission. The call and verdict are feed-visible and persisted with the real session id.

## Why not the rejected framings

- **Not a settings toggle** — cross-review is native to the agent's competence, not a user preference; the audience is non-technical.
- **Not a runtime gate** — provider CLIs spawn auto-approve by design; there is no merge/PR/approval surface in the app.
- **Not vendored bstack doctrine** — framed as a Houston product quality feature, so it dodges `CONTRIBUTING.md:13` (no external methodology in Houston docs), the non-technical voice rule, and `BROOMVA.md`'s minimal-divergence posture; potentially upstreamable.

## What v2 may add

Mount the orphaned `@houston-ai/review` package as a verdict surface; optional human-in-the-loop accept/reject (discharges the `life` `ApprovalRequired` stub at `life_runner.rs:159`); extend MCP coverage to codex/gemini sessions; a `spawn_and_monitor` reviewer that reads the repo/diff; an optional power-user `advanced.cross_review` flag for *visibility/tuning only* (default stays on).

## Tests

- `sessions::cross_review` unit tests: per-provider model defaults; `life`/unknown → no model; review-prompt JSON-escapes inputs (injection safety); focus include/omit.
- `routes::mcp` tests: `tools/list` includes `request_review` with required `work_summary`/`work_content`; missing `work_content` → `ERR_INVALID_PARAMS` before any CLI spawn; the 2-tool list assertion.
- App doctrine: `houston_prompt::system_prompt_includes_cross_review_guidance` (heading present, jargon-free).
- The actual reviewer execution needs real provider auth → validated by dogfood/manual, mirroring how `generate_instructions` unit-tests its pure logic (not the live provider call).

## See also
`docs/specs/2026-06-04-agent-cross-review.md` (the RFC) · `knowledge-base/tracker-integration.md:348` (P20 lineage) · `app/src-tauri/src/houston_prompt/` (doctrine layer) · `engine/houston-engine-core/src/sessions/generate_instructions.rs` (the one-shot pattern this mirrors) · RFC: `gethouston/houston#248`.
