# 2026-05-25 — Houston fork worktree + branch cleanup

**Outcome.** Working tree clean. Two active worktrees remain (down from 7). All cleaned-up branches were either squash-merged on the fork or had no unique commits. Eleven upstream PRs documented as in-flight; one new fork PR opened (`#60` `docs/design-system`); one new fork PR opened (this file).

## State after cleanup

- **Worktrees** (2)
  - `~/broomva/work/houston` — `fork/linear-oauth-fixes` (clean)
  - `~/broomva/work/houston/.claude/worktrees/docs-design-system` — `docs/design-system` (PR [#60](https://github.com/broomva/houston/pull/60))
- **Open PRs on `broomva/houston`** — `#60` (`docs/design-system`)
- **Open PRs on `gethouston/houston` from `broomva` fork** — 11 (see table below)
- **Local branches with `[: gone]` tracking** — 0
- **Stash** — empty

## In-flight upstream PRs (`broomva` → `gethouston`)

Merge cadence sits with upstream maintainers. Branches must stay alive locally until merged.

| PR | Branch | Mergeable | Notes |
| --- | --- | --- | --- |
| [#277](https://github.com/gethouston/houston/pull/277) | `feat/advanced-tile-layout` | CONFLICTING | rebase needed |
| [#275](https://github.com/gethouston/houston/pull/275) | `feat/ask-user-question-mcp` | OK | awaiting review |
| [#274](https://github.com/gethouston/houston/pull/274) | `feat/engine-git-routes-upstream` | OK | no local branch — pushed from another checkout |
| [#272](https://github.com/gethouston/houston/pull/272) | `chore/agent-dev-workflow` | CONFLICTING | rebase needed |
| [#271](https://github.com/gethouston/houston/pull/271) | `feat/contributor-tooling` | OK | awaiting review |
| [#269](https://github.com/gethouston/houston/pull/269) | `fix/composer-context-meter` | OK | awaiting review |
| [#267](https://github.com/gethouston/houston/pull/267) | `feat/advanced-worktrees-flag` | OK | awaiting review |
| [#263](https://github.com/gethouston/houston/pull/263) | `fix-missing-questions` | OK | awaiting review |
| [#258](https://github.com/gethouston/houston/pull/258) | `fix/claude-code-install-surfacing` | OK | awaiting review |
| [#249](https://github.com/gethouston/houston/pull/249) | `feat/engine-client-non-tauri-fallback` | OK | awaiting review |
| [#246](https://github.com/gethouston/houston/pull/246) | `docs/readme-accuracy` | OK | awaiting review |

## What was removed this session

**Worktrees (5)**
- `~/conductor/workspaces/houston/abu-dhabi` — phantom `PULL_REQUEST_TEMPLATE.md` diff caused by an upstream case-collision (see below); no real work
- `~/conductor/workspaces/houston/surabaya-v1` — identical phantom diff; no real work
- `~/broomva/work/houston/.claude/worktrees/houston-mobile-capacitor` (branch `houston-mobile-handoff`) — fork PR #43 merged
- `~/broomva/work/houston/.claude/worktrees/linear-board` (branch `fork/linear-perorg-reconcile`) — fork PR #52 merged
- `~/broomva/work/houston/.claude/worktrees/linear-mirror` (branch `fork/linear-mirror`) — fork PR #29 merged
- `~/.codex/worktrees/a665/houston` (branch `codex/a665-slash-skills`) — obsolete; same feature already merged via fork PR #58

**Local branches (11)**
- `phase-8-review-gate-rfc`, `linear-camel-case-fix` (both held only the phantom PR-template diff)
- `houston-mobile-capacitor` (merged via #28), `houston-mobile-notify-frame` (merged via #32)
- `houston-mobile-handoff` (#43), `fork/linear-perorg-reconcile` (#52), `fork/linear-mirror` (#29)
- `codex/a665-slash-skills` (superseded by #58), `feat/advanced-slash-skills` (cherry-pick attempt aborted; feature already merged)
- `auckland`, `medan`, `hong-kong`, `valencia` — Conductor city-named orphans pointing at upstream-merged commits (`4a0f32e`, `9162932`); no remote, no unique commits

## What was added this session

- New file `DESIGN.md` (~280 lines) — comprehensive design-system spec at repo root → fork PR [#60](https://github.com/broomva/houston/pull/60) on branch `docs/design-system`
- This handoff doc → fork PR (current branch `docs/fork-cleanup-2026-05-25`)

## Upstream issue to file

`origin/main` (tip `512d2ab`) has a **case-collision** in the Git index: both `.github/PULL_REQUEST_TEMPLATE.md` (blob `3f8ae17`) and `.github/pull_request_template.md` (blob `9309346`) are tracked. On macOS case-insensitive filesystems only one can sit on disk, so the other shows as a persistent phantom modification that `git restore` cannot fix — only one of the two index entries can match disk content at a time. Upstream needs to `git rm` one of the two paths in `origin/main`.

## Candidates for further cleanup (NOT done this session)

Each requires a quick sanity check before deletion — listing so a future session can pick them up.

**Fork-prefix duplicates with their parallel upstream branch still open** — keep both for now:
- `fork/feat/advanced-tile-layout` (parallel `feat/advanced-tile-layout` → upstream PR #277)
- `fork/feat/ask-user-question-mcp` (parallel `feat/ask-user-question-mcp` → upstream PR #275)
- `fork/fix-missing-questions` (parallel `fix-missing-questions` → upstream PR #263)
- `fork/docs/git-workflow-fork-first` (parallel `docs/git-workflow-fork-first` — no upstream PR)

**Fork-merged, no upstream parallel** — safe to delete locally:
- `fork/a665-slash-skills` (#58), `fork/chore/drop-prompt-md` (#59), `fork/feat/advanced-claude-hooks` (#54), `fork/linear-agent-session` (#35), `fork/linear-board` (#30), `fork/linear-kb-docs` (#36), `fork/linear-perorg-routes` (#51), `fork/linear-tab` (#37), `fork/linear-tracker-spec` (#21), `fork/linear-webhook` (#34), `fork/linear-workspace-scope` (#44), `fork/linear-workspace-ui` (#49)

**Non-prefixed fork-merged, no open upstream PR** — verify before deleting:
- `chore/fork-attribution` (#7), `chore/gitignore-harness-artifacts` (#16), `chore/sync-upstream-action` (#8)
- `docs/agent-dogfood-loop` (#6), `docs/broomva-md-bun-refresh` (#26), `docs/dogfood-pattern-adoption` (#11), `docs/platform-dogfood-pattern` (#12)
- `feat/advanced-context-meter` (#20), `feat/advanced-settings-infra` (#10), `feat/banned-patterns-lint` (#14), `feat/check-cli-deps` (#3), `feat/ci-pr-workflow` (#1), `feat/houston-doctor` (#2), `feat/migrate-to-bun` (#9)

**Carries unmerged commits — investigate before doing anything**:
- `docs/wop-spec` — ahead 12, behind 1 vs `fork/docs/wop-spec`
- `claude/linear-mirror` — ahead 3, behind 30 vs `fork/main`
- `feat/advanced-claude-hooks` — ahead 1 vs `fork/feat/advanced-claude-hooks`

**Admin / sync labels** — review purpose:
- `chore/pr-consolidation`, `fork-main-unified`, `main`, `claude/git-versioning-oss-pm`, `claude/linear-board`

## Next-session order of operations

1. Rebase the two CONFLICTING upstream PRs (#272, #277) once you have spare cycles; force-push.
2. Nudge upstream maintainers on any of the 9 MERGEABLE upstream PRs that have been idle > 7 days.
3. Delete the "fork-merged, no upstream parallel" block (12 branches, zero-risk after PR cross-check).
4. Verify and delete the "non-prefixed fork-merged" block (14 branches).
5. Investigate the three "carries unmerged commits" branches; either PR them or rebase-then-discard.
6. File the upstream `PULL_REQUEST_TEMPLATE.md` case-collision issue against `gethouston/houston`.
