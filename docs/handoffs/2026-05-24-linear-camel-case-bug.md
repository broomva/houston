# Handoff — Linear `TrackerIssue` snake_case ↔ camelCase serialization bug

**Date**: 2026-05-24
**Severity**: User-visible — every Linear issue lands in the "To do" column of the LinearView kanban regardless of actual state
**Discovered**: dogfooding the V1 Linear vertical end-to-end with 1232 real issues from user's Linear org
**Where**: `engine/houston-engine-protocol/src/lib.rs::TrackerIssue` (the canonical struct, used both for wire AND on-disk persistence)

## State of the V1 Linear vertical (cumulative this session)

10 PRs merged on `fork/main` of `broomva/houston` (fork-first cadence — no upstream PRs opened):

| # | Chunk | What |
|---|---|---|
| #30 | C13 | LinearIssuesList preview inside Settings → Tracker Connected card |
| #34 | C3 | Webhook HMAC-SHA256 + replay window + idempotency ledger |
| #35 | C4 | AgentSession protocol (inbox handoff + `agentActivityCreate` mutation) |
| #36 | C16 | KB cross-references (architecture / engine-protocol / files-first / tracker-integration) |
| #37 | C14 | Full Linear board kanban view in agent shell (sidebar nav entry) |
| #44 | PR A | Workspace-many foundation — `ConnectionMeta::list_for_workspace`, migration helper, `/connections` route, TS types |
| #49 | PR B | UI panel surfacing the workspace-many list (informational, hidden when ≤1) |
| #51 | PR C | Per-org disconnect routes + per-row Disconnect button (with 1 follow-up fix for Linux-keychain edge case) |
| #52 | PR D | Per-org reconcile + per-row Sync + LinearView connection picker |

**Pending** chunks (user-blocked or out-of-scope):
- **PR E** — retire legacy per-agent path (destructive; awaits user direction)
- **C4b** — engine-core file-watcher → dispatch (closes ingress loop)
- **C7** — routing.json policy (workspace inbox → specific agent)
- **C11** — `houston-relay` Cloudflare Worker `/linear/webhook/{tunnelId}` deploy
- **C15** — `bug_report/linear*.rs` migration (cross-boundary design decision, deferred)
- **C17** — E2E + canonical V1 dogfood receipt (this session is most of it; just needs codification)

## The bug — concretely

### Symptom

Open Houston → Settings → Tracker (or sidebar `Linear`) → kanban shows **all 1232 issues in "To do"**. None in "In progress" or "Done", despite the user's Linear org having states across the full lifecycle.

### Root cause

`engine/houston-engine-protocol/src/lib.rs` defines `TrackerIssue` with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]   // ← this
pub struct TrackerIssue {
    pub provider: String,
    pub provider_id: String,
    pub state_type: Option<String>,
    pub team_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub assigned_houston_agent_id: Option<String>,
    // ...
}
```

So on-disk JSON looks like:
```json
{"provider":"linear","provider_id":"...","state_type":"completed","team_id":"...","created_at":"..."}
```

But the TS type in `ui/engine-client/src/types.ts::TrackerIssue` declares camelCase:
```typescript
export interface TrackerIssue {
  provider: TrackerProvider;
  providerId: string;          // expects providerId, JSON has provider_id
  stateType?: string;          // expects stateType, JSON has state_type
  teamId: string;              // ...
  createdAt: string;
  updatedAt: string;
  // ...
}
```

Result: when the engine returns `issues.json` over `GET /v1/trackers/linear/issues`, the TS frontend reads `state_type` as `undefined`. The `LinearKanban` mapping:

```typescript
status: issue.stateType ?? "unstarted",   // always falls into the ?? branch
```

→ every issue gets `status: "unstarted"` → matches only the `todo` column's `statuses: ["triage", "backlog", "unstarted"]` → all 1232 stack there.

### Why the rest of the protocol DTOs are fine

Other tracker DTOs in `protocol/lib.rs` use `#[serde(rename_all = "camelCase")]`:
- `TrackerConnectRequest` ✓ camelCase (`workspacePath`, `clientId`)
- `TrackerConnectResponse` ✓ camelCase (`authorizeUrl`, `callbackPort`)
- `TrackerStatusResponse` ✓ camelCase (`orgId`, `orgName`, `lastSyncAt`)
- `TrackerConnectionList` / `TrackerConnectionListItem` ✓ camelCase (PR A)
- `TrackerReconcileResponse` ✗ also `snake_case` (`issues_seen`, `pages_fetched`, `cursor_advanced_to`) — **same bug, also needs fix**

`TrackerIssue` was the odd one out — likely because it was originally designed for on-disk persistence first (where snake_case matches Linear's own GraphQL field naming convention) and then later wired to the wire layer without rechecking.

## Fix design

The struct serves both on-disk AND wire purposes (documented in its own doc comment: *"Wire shape used by both the engine route response AND the on-disk persistence"*). Both layers need to agree.

### Option A — Change to `rename_all = "camelCase"` + migration (recommended)

```rust
#[serde(rename_all = "camelCase")]
pub struct TrackerIssue {
    pub provider: String,
    pub provider_id: String,
    pub state_type: Option<String>,
    // ...
}
```

**Pros**:
- Aligns with the rest of the protocol + TS types
- No TS changes needed
- One source of truth (`#[serde(rename_all = "camelCase")]` consistently across all tracker DTOs)

**Cons**:
- Existing on-disk `issues.json` + 1232 raw/issues/*.json files won't deserialize (snake_case fields no longer match)
- Need migration

**Migration approaches** (pick one):

1. **`#[serde(alias = "snake_case_name")]` on each field** — accept both shapes on input, always emit camelCase on output. After first reconcile, files rewrite to camelCase. Zero data loss; backward compatible during transition. **Cleanest.**

2. **Detect + re-fetch** — on load_projection failure, treat as empty and trigger reconcile. Works but burns rate-limit budget for users with large mirrors.

3. **Manual migration helper** — `migrate_issues_snake_to_camel(workspace_path)` reads existing files, re-parses with old shape, re-serializes with new. Runs once on engine boot if it detects snake_case files.

Recommended: **option 1 (serde alias)**. Quick, zero data loss, no engine-boot work. The aliases can be removed in a follow-up PR after all users have soaked the new shape.

### Option B — Keep on-disk snake_case, wire-only camelCase

Use `#[serde(rename_all_fields = "camelCase")]` only at the route boundary by wrapping in a thin DTO. More moving parts; not recommended unless on-disk snake_case has some other constraint we don't know about.

### Option C — Change TS to snake_case

Cargo-cults the rest of the codebase the wrong way. Don't.

## Files to touch (PR scope)

### Engine

- `engine/houston-engine-protocol/src/lib.rs`:
  - `TrackerIssue`: change `#[serde(rename_all = "snake_case")]` → `#[serde(rename_all = "camelCase")]`
  - Add `#[serde(alias = "<snake_case>")]` per multi-word field for backward compat
  - Same treatment for `TrackerReconcileResponse` (also has snake_case fields: `issues_seen`, `pages_fetched`, `cursor_advanced_to`)

- `engine/houston-linear/src/models.rs`:
  - No struct changes (it just uses `TrackerIssue`) but **add a test** that exercises the snake-case-alias load path so we don't regress

### Tests

- Add `engine/houston-engine-protocol/src/lib.rs::tests::tracker_issue_serializes_camel_and_deserializes_either`:
  - Build a `TrackerIssue`, `serde_json::to_string` → assert keys are camelCase (`providerId`, `stateType`)
  - Build a snake_case JSON string by hand, `serde_json::from_str::<TrackerIssue>` → assert it parses
  - Build a camelCase JSON string, `serde_json::from_str::<TrackerIssue>` → assert it parses

- Update `engine/houston-linear/src/models.rs::tests::projected_issue_serializes_to_schema_shape`:
  - Change assertions from `json["provider_id"]` → `json["providerId"]`
  - Change `json["state_type"]` → `json["stateType"]`
  - Change `json["assigned_houston_agent_id"]` → `json["assignedHoustonAgentId"]`

- Add `engine/houston-linear/src/models.rs::tests::load_projection_accepts_legacy_snake_case`:
  - Write a synthetic snake_case `issues.json` to a TempDir
  - Call `load_projection(dir.path())`
  - Assert the issue parsed correctly with state_type populated

### UI

- Probably **no TS changes** if the alias approach is taken — TS types already expect camelCase. Validate via:
  - `bunx tsc --noEmit` (will fail if any TS expects snake_case fields)
  - Visual check: kanban now distributes issues across the three columns

### Schemas

- `ui/agent-schemas/src/tracker_issue.schema.json`:
  - Currently declares snake_case keys to match the on-disk shape
  - Change to camelCase to match the new wire/disk shape
  - Bump version + add the snake_case shape as an `oneOf` for backward compat during migration if schema validation is enforced anywhere

### Docs

- `knowledge-base/tracker-integration.md`: under "Filesystem layout" the `issues.json` content example (if any) should use camelCase. Cross-ref to this fix.
- `docs/specs/2026-05-23-tracker-integration.html` — same; update inline JSON samples.

## Validation steps for the fix PR

```bash
# 1. Touch only the files above. cargo test should be green:
cargo test -p houston-linear -p houston-engine-protocol

# 2. Backward-compat test passes (loads legacy snake_case file):
cargo test -p houston-linear models::tests::load_projection_accepts_legacy_snake_case

# 3. TS still compiles without changes:
cd app && bunx tsc --noEmit

# 4. Locale parity unchanged:
cd app && bun run check-locales

# 5. cargo fmt:
cargo fmt -p houston-linear -p houston-engine-protocol

# 6. Rebuild + restart Houston dev to pick up the new sidecar:
cd /Users/broomva/broomva/work/houston
cargo build -p houston-engine-server
# Kill any running houston-engine / houston-app / vite / bun-tauri processes
pkill -f "houston-engine|houston-app|vite|bun.*tauri"
sleep 3
cd app && LINEAR_CLIENT_ID=$LINEAR_CLIENT_ID LINEAR_CLIENT_SECRET=$LINEAR_CLIENT_SECRET bun run tauri dev
# Wait for engine to spawn, then open Houston → Linear tab.
# Issues should distribute across To do / In progress / Done columns.
```

**Dogfood receipt**: take a screenshot showing the kanban with issues split across at least two columns. Attach to the PR body.

## Repo / branch state at handoff

- Main checkout: `/Users/broomva/broomva/work/houston/` on `fork/main` at SHA `566b5a6` (latest as of handoff write time)
- Worktree: `/Users/broomva/broomva/work/houston/.claude/worktrees/linear-board/` on `fork/linear-perorg-reconcile` (PR D branch, already merged + tracking stale fork-side; safe to delete)
- Dev shell env vars persisted in `~/.zshrc`:
  - `LINEAR_CLIENT_ID=80df31c934ce332915f1a77f7525d30e`
  - `LINEAR_CLIENT_SECRET=2800e228a41e9fc62a39cf8c4ecba9c5`
- Dev houston root: `~/.dev-houston/` (separate from prod `~/.houston/`)
- Connected Linear data: `~/.dev-houston/workspaces/BstackPort/Broomva/.houston/trackers/linear/`
  - `connection.json` (org_id, capabilities)
  - `issues.json` (2.6 MB, snake_case) ← will be re-written to camelCase after fix + next sync
  - `raw/issues/*.json` (1232 files, snake_case) ← same
  - `sync_state.json` (cursor for incremental sync)
- macOS keychain entry: `Houston-Linear-credentials` (OAuth tokens for `broomva` Linear org)

## Reflexive context for the next agent

- **Worktree workflow** per `CLAUDE.md`: every task in its own `.claude/worktrees/<name>/` branch; never touch `claude/wip` or `main`.
- **Fork-first git** per `CLAUDE.md`: cherry-pick onto a `fork/<name>` branch off `fork/main`, push to fork remote, `gh pr create --repo broomva/houston --base main --head fork/<name>`, `gh pr merge --squash --delete-branch --auto`. **Never push the worktree branch to `origin`.**
- **No silent failures** per `CLAUDE.md`: every error a user-initiated action can produce reaches the user as a visible toast.
- **i18n** per `CLAUDE.md`: every user-facing string via `t()`; en/es/pt locales; no em-dashes in user copy.
- **File size**: 200 lines/file (excluding tests).

## Other small follow-ups discovered during this session

- **Engine OAuth completion still writes per-agent** (`commands::task::run_connect_task` → `ConnectionMeta::write_atomic` (legacy)). PR A added the workspace-many helpers but didn't switch the OAuth flow over. Currently the dev install has the connection.json under the agent dir, not the workspace `/connections/` dir. PR E (or a small precursor PR) should switch OAuth completion to use `write_atomic_for_workspace`.
- **`webhook_events.jsonl` is empty** because C11 relay isn't deployed. Once a Linear webhook reaches the engine, the existing C3 ledger + C4 dispatch + (future C4b) file-watcher chain takes over.
- **Per-org `seed_layout` not called on OAuth completion** — per-org `raw/` + projection dirs lazy-create on first reconcile write. Works but `agent_sessions/` stays missing until first AgentSession event. Cosmetic; can fold into PR E.
