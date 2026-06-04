# RFC — Native agent cross-review (Phase 8)

**Status:** v1 implemented on branch `advanced-review-gate-phase8` (fork-only). Direction approved product-native (§7 D4). See "Implementation status" below.
**Date:** 2026-06-04
**Lineage:** Phase 8 of the advanced-settings wave (upstream tracking issue RFC #248), **reframed**. The original task-list name was `advanced.review_gate` / "P20 cross-review enforcement." That framing is rejected here (§2); this RFC replaces it.
**Branch:** `advanced-review-gate-phase8` (fork-only).

## Implementation status (v1 as built)

Shipped behind the design below, with three refinements found during implementation grounding (all simplifications):

- **Capability is stateless.** `houston_engine_core::sessions::cross_review::run_review(caller, work_summary, work_content, focus)` is a free function (mirrors `generate_instructions`), wrapping `provider_oneshot::run_provider_oneshot`. No session-context registry, no `run_start`/`cancel` lifecycle wiring — the original RFC over-scoped this.
- **Visibility is automatic.** The `request_review` tool call and its result already ride the NDJSON parser rails into the session feed (every non-`AskUserQuestion` tool passes through unchanged), so v1 needs no explicit `FeedItem`/DB emission. An explicit "I got a second opinion" line is v2 polish.
- **Reviewer fallback is self-review, not skip (refines D6).** Prefer an authenticated provider different from the caller (cheap `provider::check_status` pre-check, no session spawn); if none usable, fall back to a fresh-context **same-provider** review (`crossProvider: false`) so the agent always gets a second opinion and the capability never silently fails. Caller is anthropic in v1 (MCP is anthropic-only, `sessions/mod.rs:454`); documented v2 upgrade reads the caller from context.

**Verified:** engine `cargo test` green — `sessions::cross_review` 5/5, `routes::mcp` 10/10 (incl. `tools_list_includes_request_review`, `request_review_missing_work_content_is_invalid_params`, and the updated 2-tool list assertion). App doctrine (`houston_prompt`) verified by standalone compile of the exact const + `system_prompt()` `format!` + test; full `cargo test --manifest-path app/src-tauri/Cargo.toml houston_prompt` still owed once disk space frees (a system-wide full disk blocked the Tauri-crate build, not the code).

---

## 1. Summary (TL;DR)

Give Houston agents the ability to **get a second opinion from a different AI model on important work, by their own judgment, before presenting it as done** — and bake that behavior into every Houston agent by default.

This is the productized, de-jargoned realization of bstack **P20 Cross-Review** (`knowledge-base/tracker-integration.md:348`: "substantial work gated through cross-model adversarial review"). It is **not** a user-facing settings toggle and **not** a hard runtime gate. It is **agent doctrine + an engine capability**.

Two parts, both required:

- **Part A — Capability (engine).** A new in-engine MCP tool the running agent can call to run a governed, visible cross-model review. Today the agent *cannot* do this through any supported primitive.
- **Part B — Doctrine (product prompt).** A new section in Houston's global product prompt that makes the agent self-trigger the capability *generatively* when the work warrants it, phrased in plain non-technical voice.

Ship A without B → the tool exists but nothing uses it. Ship B without A → the agent is told to review but can only fake it or Bash-hack it (ungoverned, invisible). Phase 8 ships both.

---

## 2. Motivation & the rejected framing

### Why this is worth doing
A non-technical Houston user can't tell whether the agent's output is right. The product's quality ceiling is "whatever one model produced in one pass." P20's insight — a *different* model adversarially checking the work catches errors the author model is blind to — is exactly the lever that raises that ceiling without asking the user to verify anything. The agent does the rigor; the user gets a better result.

### Why "advanced.review_gate" / "P20 cross-review enforcement" is rejected
Three independent grounding sweeps (saved to session memory) established:

1. **P20 is a dev-process discipline, not a runtime feature.** Every in-repo reference (`knowledge-base/tracker-integration.md:348`, `docs/development/dogfood-pattern.html:531`) is about how *PRs* get reviewed before *merge*. Houston the app has no merge, no PR, no runtime "review gate."
2. **A settings toggle is the wrong layer.** Cross-review is native to the agent's competence, not a user preference. The user agreed: it is not a fit for a non-technical user. Exposing it as `advanced.*` (`app/src/lib/featureFlags.ts:54-151`) would surface engineering jargon to the wrong audience.
3. **There is no runtime gate to enforce.** Provider CLIs are spawned auto-approve by design (`gemini --yolo`, `codex --dangerously-bypass-approvals-and-sandbox`); the only "ApprovalRequired" concept is the experimental fork-only `life` provider, stubbed at `life_runner.rs:159-165` ("Houston has no review-queue UI yet").

So Phase 8 moves from `featureFlags.ts` (wrong) to **the agent doctrine layer + a real engine capability** (right).

---

## 3. Non-goals

- **Not** a user-facing settings toggle. Default behavior, embedded, not opt-in. (A *power-user visibility/tuning* flag is a possible v2 — §8 — never the enable switch.)
- **Not** a hard human-in-the-loop gate in v1. The review is advisory input the agent weighs; it does not block the turn waiting for a human to click approve. (HITL gate is a v2 option — §8.)
- **Not** importing bstack/P-number jargon into shipped agent voice or Houston docs (respects `CONTRIBUTING.md:13` + the non-technical voice rule). "P20" appears only here, as lineage.
- **Not** review-of-the-user. This is the agent reviewing *its own* work via a peer model, never a quality judgment on the human.

---

## 4. Background: the two layers this builds on

### 4.1 Where default agent behavior lives (Part B target)
- An agent's effective system prompt is `<product_prompt>\n\n---\n\n<agent_context>` (`engine/houston-engine-core/src/sessions/mod.rs:265-280`).
- `product_prompt` is Houston's global doctrine layer, assembled in `app/src-tauri/src/houston_prompt/mod.rs:22-26` (`system_prompt()`) from `base.rs` (identity, interaction loop, approval gate) + `skills_memory.rs` + `routines.rs` + `integrations.rs`, handed to the engine via `HOUSTON_APP_SYSTEM_PROMPT` (`app/src-tauri/src/lib.rs:243-247`).
- It reaches **every** agent and **every** provider uniformly: claude `--system-prompt` (`claude_runner.rs:210-211`), codex `developer_instructions` (`codex_command.rs:21-25`), gemini inlined `<system>` (`gemini_runner.rs:264-269`).
- It is compiled into the app, **not** user-editable, and survives agent (re)creation — unlike a seeded `CLAUDE.md` (`DEFAULT_CLAUDE_MD` at `agents/prompt.rs:89-98` is write-once, bespoke-per-agent, and overwritten for store agents). **This is the only correct home for a universal default behavior.**

### 4.2 What cross-model execution the engine can already do (Part A backing)
- `engine/houston-engine-core/src/sessions/provider_oneshot.rs:30` — `run_provider_oneshot(prompt, provider, model, timeout)`: one-shot prompt against any provider, returns stdout. (Already used host-side for titles/compaction.)
- `engine/houston-agents-conversations/src/session_runner.rs:89` — `spawn_and_monitor(...)`: full tool-using provider session.
- Multi-provider registry (`provider/mod.rs:194,240`): anthropic, openai, gemini, life — a `Copy` newtype over a static adapter.
- **Gap:** all of this is engine-internal Rust. The agent's *only* callable MCP tool is `AskUserQuestion` (`mcp.rs:73`, `:249`, `:305`), and MCP is wired **only for anthropic sessions** (`sessions/mod.rs:454`). The agent has Bash + `--dangerously-skip-permissions` (`claude_runner.rs:188`) but no engine credentials (`claude_runner.rs:178` injects only PATH; routes are bearer-gated at `lib.rs:52-54`). So today cross-review is only possible as an ungoverned, invisible Bash shell-out.

### 4.3 Existing scaffolding this discharges
- `ui/review/*` (`@houston-ai/review`) — a complete master/detail review UI (`ReviewSplit`, `DeliverableCard`, `onApprove`/`onReject`), **built but never mounted** (only ref: `app/src/styles/globals.css:13`).
- `life` `ApprovalRequired`/`ApproveDispatch` proto (`engine/houston-life/proto/life/v1/agent.proto:27-29,89`) + its stubbed handler. v2 can wire these; v1 does not depend on them.

---

## 5. Design

### Part A — `request_review` engine capability

A new tool registered in the in-engine MCP server (`engine/houston-engine-server/src/routes/mcp.rs`), alongside `AskUserQuestion`.

- **Shape (v1):** `request_review({ work_summary, work_content, focus? }) -> { verdict, issues[], suggestions[] }`. The agent passes a summary of what it did + the artifact/content it wants checked + an optional focus hint.
- **Reviewer selection:** the tool picks a reviewer model on a **different provider than the calling agent** (the essence of cross-*model*; never self-review with the same model). Resolved via `Provider::from_str` against the registry. If no alternative provider is configured/authenticated, the tool returns a clean "no reviewer available" result so the agent degrades gracefully (never a silent failure — beta policy).
- **Backing (v1):** `run_provider_oneshot` — feed the reviewer a critique rubric + the work, get a structured verdict. Cheap, stateless, no working-dir access. (v2: optional `spawn_and_monitor` reviewer that can read the repo/diff itself.)
- **Governance:** runs in-engine, with the engine's own credentials and the agent's working dir context — not a credential-less shell-out. Bounded by a timeout.
- **Visibility:** emits a `FeedItem` / chat event so the review is recorded and inspectable (the Bash hack is invisible; this must not be). Returns the verdict to the agent as a `tool_result` via the same mechanism `AskUserQuestion` uses (`mcp.rs:341`).
- **MCP coverage (v1):** anthropic sessions only (the default provider; the only one with an `mcp_config` today, `sessions/mod.rs:454`). Codex/gemini coverage is §6 D2.

### Part B — cross-review doctrine in the product prompt

A new module `app/src-tauri/src/houston_prompt/cross_review.rs` exporting a `CROSS_REVIEW_GUIDANCE` const, wired into `system_prompt()` (`mod.rs:22-26`) mirroring the existing `routines.rs` / `skills_memory.rs` split (keeps every file well under the 200-line limit).

It instructs the agent, **in plain product voice** (no "P20", "cross-model", "adversarial", or any file/JSON/CLI mention — per `base.rs` voice rules):

- *When* to self-review — generatively, by judgment: work the user will rely on, anything substantial or hard to undo, drafts/analysis/decisions with real stakes. **Not** every trivial turn.
- *How* — use the second-opinion capability, weigh the feedback honestly, fix real problems, and briefly tell the user if a check changed the result.
- *Tone* — this is the agent being diligent, invisible plumbing to the user ("I double-checked this"), never exposing the machinery.

The doctrine is generative guidance, not a hard rule — the agent decides when it's warranted, which is exactly the user's ask: "P20 happens natively and generatively as sessions occur and the agent decides it's needed."

---

## 6. Key decisions (recommendations inline)

| # | Decision | Options | Recommend |
|---|----------|---------|-----------|
| **D1** | Reviewer depth | one-shot text verdict / full tool-using reviewer session | **one-shot (v1)**; `spawn_and_monitor` reviewer in v2 |
| **D2** | MCP coverage | anthropic-only / all providers | **anthropic-only (v1)**; extend in v2 (needs `mcp_config` for codex+gemini) |
| **D3** | Advisory vs gate | advisory input / blocks turn for human approve-reject | **advisory (v1)**; HITL gate via mounting `@houston-ai/review` is v2 |
| **D4** | Framing (strategic) | product-native "second opinion" / explicit bstack-P20 doctrine in agents | **product-native** — see §7 |
| **D5** | Standalone-engine coverage | doctrine in app prompt only (engine-only deploys miss Part B) vs duplicate into engine | **app prompt for B, engine for A.** Engine-only consumers ship their own doctrine; the capability tool still works for them |
| **D6** | Self-review policy | allow same-model / require different provider | **prefer different authenticated provider; fall back to same-provider fresh-context review** when none usable (refined during build — see Implementation status) |

**D4 is the one I cannot decide for you** — it is a strategy/values call (§7).

---

## 7. Fork-norm tension (needs your call)

Embedding operating doctrine into shipped Houston agents runs into stated norms:

- `CONTRIBUTING.md:13` — "Don't add your own methodology, RFCs, or doctrine to Houston's `knowledge-base/` or `docs/`."
- `BROOMVA.md:5-16` — the fork carries "as few divergent patches as possible"; bstack primitives are kept as upstream **RFC discussions** (#243 bstack primitives, #255 adopt P11), never merged doctrine.
- Non-technical agent voice rule (CLAUDE.md) — "P20"/"cross-review" jargon doesn't belong in agent-facing copy.

**How the recommended framing (D4 = product-native) resolves all three:** ship this as a Houston **product quality feature** — "agents get a second opinion on important work" — inspired by P20 but not vendoring bstack vocabulary or doctrine. Then:
- No primitive jargon reaches users or Houston docs → no `CONTRIBUTING.md:13` conflict (this RFC is a Houston *feature spec*, a peer of `docs/specs/2026-05-23-tracker-integration.html`, not imported methodology).
- It's a genuine, potentially **upstreamable** feature → aligns with BROOMVA.md's "contribute features up, don't carry doctrine" posture, and is the de-jargoned path that upstream RFC #243 could actually accept.
- Plain voice satisfies the non-technical rule.

The alternative (explicit bstack-P20 doctrine in agents) is heavier divergence, conflicts with the above, and gains nothing the product-native framing doesn't. **Recommend product-native.** Confirm before drafting code.

Per your "work on the fork only" instruction, v1 lands fork-side regardless; product-native framing just keeps the upstream door open.

---

## 8. Phasing

- **v1 (this RFC):** Part A `request_review` (anthropic, one-shot, governed, visible FeedItem, different-provider reviewer) + Part B `cross_review.rs` doctrine. Advisory. Tests. KB doc.
- **v2 (separate RFC):** mount `@houston-ai/review` as the verdict surface; optional HITL accept/reject (discharges the `life` `ApprovalRequired` stub); extend MCP to codex/gemini; `spawn_and_monitor` reviewer that reads the diff; optional power-user `advanced.cross_review` flag for *visibility/tuning only* (default behavior stays on) — which would then follow the `advanced.*` shape (`featureFlags.ts` + the `claude_hooks` engine-enforced template at `routes/claude_hooks.rs:24-60`).

---

## 9. Files touched (v1, estimated)

**Engine (capability):**
- `engine/houston-engine-server/src/routes/mcp.rs` — register `request_review`; tools/list; dispatch; tool_result.
- `engine/houston-engine-core/src/sessions/` — a `cross_review` module wrapping `provider_oneshot` + reviewer-provider selection + FeedItem emit. (Possibly a leaf crate if it grows.)
- `engine/houston-engine-core/src/sessions/mod.rs:454` — confirm/adjust the anthropic `mcp_config` wiring.
- Wire types if the tool result needs a typed DTO (`houston-engine-protocol` + `ui/engine-client/src/types.ts`).

**App (doctrine):**
- `app/src-tauri/src/houston_prompt/cross_review.rs` — new const.
- `app/src-tauri/src/houston_prompt/mod.rs:8-26` — `mod`/`pub use`/concat into `system_prompt()` + a unit test in the existing `tests` block.

**Docs:**
- `knowledge-base/` — a feature doc (product-voiced; mirrors the `knowledge-base/advanced-*.md` shape but this is not an advanced flag, so likely `knowledge-base/agent-cross-review.md`).

## 10. Tests
- `houston_prompt` unit test asserting `CROSS_REVIEW_GUIDANCE` is present and jargon-free (no "P20"/"cross-model"), alongside `mod.rs:37-92`.
- Engine: `request_review` returns a verdict for a stub provider; selects a *different* provider than the caller; degrades cleanly when no alternative provider is authenticated (no silent failure); emits a FeedItem.
- Reviewer timeout path surfaces an error (beta no-silent-failure policy).

## 11. Open questions
- Reviewer rubric: fixed critique template vs agent-supplied `focus`? (Lean: fixed rubric + optional focus.)
- Cost/latency budget for a default-on second pass — token + wall-clock ceiling per review; should the doctrine cap reviews per turn?
- Does `request_review` need the work artifact inline, or a pointer the engine reads (e.g. a file path in the working dir)? Inline is simpler for v1 one-shot.
- Visibility surface: a plain chat/FeedItem line in v1, or hold for the `@houston-ai/review` mount in v2?

## 12. Alternatives considered
- **Pure prompt doctrine, no capability** — agent told to review, executes via ungoverned Bash shell-out. Rejected: invisible, credential-less, unscaffolded, no different-model guarantee.
- **Hard runtime HITL gate** (block every turn until human approve/reject) — rejected for v1: doesn't fit the non-technical, low-friction product; nothing to gate today; heavier than the value warrants. Available as v2 opt-in.
- **Close Phase 8 / do nothing** — the honest fallback if the second-opinion feature isn't wanted. Rejected here because the capability is genuinely valuable and discharges existing debt (`@houston-ai/review`, `life` stub).

---

**See also:** `knowledge-base/tracker-integration.md:348` (P20 lineage) · `app/src-tauri/src/houston_prompt/` (doctrine layer) · `engine/houston-engine-server/src/routes/mcp.rs` (capability layer) · upstream RFC #248 (advanced-settings wave), #243 (bstack primitives).
