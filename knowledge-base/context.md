# Context (workspace vs agent) + enablement

How Houston defines context, how agents are isolated, and how a blank context
gets bootstrapped from the user's existing knowledge.

## The model: two layers, three scopes

Final prompt the provider CLI sees = `<product_prompt>` + `---` + `<agent_context>`.

| Scope | Lives in | Shared by | Edited by | Injected by |
|---|---|---|---|---|
| **Product** | `app/src-tauri/src/houston_prompt/` (compiled into the binary) | every user + agent | nobody | env `HOUSTON_APP_SYSTEM_PROMPT` |
| **Workspace** | `<ws>/WORKSPACE.md` (company/project) + `<ws>/USER.md` (the human) | every agent in the workspace | user (Settings) + agent (carve-out) | `workspace_context::build_prompt_section` |
| **Agent** | `<agent>/CLAUDE.md` (job desc) + `.houston/learnings/` + `.houston/prompts/modes/` + `.agents/skills/` + `.houston/integrations.json` | that one agent | user (tabs) + agent (within root) | `build_agent_context` (CLAUDE.md read by the CLI itself) |

Assembly: `engine/houston-engine-core/src/agents/prompt.rs::build_agent_context`.
Workspace section: `engine/houston-engine-core/src/workspace_context.rs`.

`WORKSPACE.md` / `USER.md` are **never seeded** — they exist only once written.
Until then the prompt renders `(empty so far …)` so the agent knows the slot
exists and is authorized to fill it.

## Isolation (how agents stay separate)

1. **Filesystem wall.** First block of every agent prompt is
   `# Working Directory — MANDATORY`: all file I/O restricted to
   `<agent-root>` (`~/.houston/workspaces/{WS}/{Agent}/`). Each agent has its own
   root, so agents cannot read each other's files.
2. **The one deliberate hole.** `build_prompt_section` injects the parent-dir
   `WORKSPACE.md` + `USER.md` and explicitly authorizes read/write of those two
   paths as an exception to the working-directory rule. Nothing else in the
   parent is reachable. This is how workspace context is shared without breaking
   isolation.
3. **Session ownership.** A per-`working_dir` guard means one session owns a
   directory at a time; a second session in the same folder gets a conflict, not
   a false file-attribution. Different agents/worktrees run in parallel.

Workspace marker = presence of `<ws>/.houston/`. No marker → `build_prompt_section`
returns `None` (test dirs and ad-hoc working dirs stay clean).

## Enablement (filling a blank context)

Module: `engine/houston-engine-core/src/context_bootstrap/`. Design spec:
`docs/specs/2026-06-06-context-enablement.html`.

Import-first, ask-residuals-last. The flow:

1. **Import** — `POST /v1/workspaces/:id/context/import` with
   `{ sources: [{ kind, path }] }`. Five source kinds (`ImportSourceKind`):
   `localFolder`, `claudeHome` (`~/.claude` memory), `obsidianVault`,
   `chatGptExport`, `claudeAiExport`. Each is parsed into one bounded, redacted
   corpus staged at `<ws>/.houston/context-import/corpus.md`. Per-file problems
   are reported in `ImportSummary.skipped` (never silently dropped); a
   whole-source failure is a hard error (toast).
2. **Synthesize** — `POST /v1/workspaces/:id/context/synthesize`
   (`{ provider?, model? }`). Runs the staged corpus through the user's own
   provider CLI (reusing `sessions::provider_oneshot`) to draft `USER.md` +
   `WORKSPACE.md` plus residual questions. Produces no activity card.
3. **Residual questions** — `ResidualQuestion { id, prompt, slot, kind }`.
   `kind = content` is a missing fact; `kind = sourceHint` asks the user where
   richer material lives and, when answered with a path, loops back into import.
   When the user skips import entirely, the UI falls back to a fixed question set.
4. **Review + write** — the user edits the draft, answers questions, then the UI
   assembles the final markdown and calls the **existing** `PUT .../context`.
   Nothing is persisted until the user approves.

Bounds + safety (`context_bootstrap/limits.rs`, `redact.rs`): extension
allowlist, file-count / byte / token caps, obvious-secret redaction before
staging. All processing is local; the corpus never leaves the machine.

### App surface

- Native pickers: `pick_directory` / `pick_file` (Tauri, `commands/os.rs`) →
  `osPickDirectory` / `osPickFile` (`app/src/lib/os-bridge.ts`).
- Wizard: `app/src/components/context-setup/` (Dialog: source chooser → import →
  synthesize → review → done). i18n namespace `contextSetup` (en/es/pt).
- Entry points: Settings → Shared context ("Set up automatically" / "Improve
  with my content") and the onboarding Summary screen CTA.

### Voice

UI never says `USER.md` / files / JSON — it says "tell your assistant about you
and your work." Product-voice invariant (agents' users are non-technical).
