# Advanced: Claude Code hooks

`advanced.claude_hooks` — Phase 7 of RFC #248. Installs Houston-managed
hook entries into the user's `~/.claude/settings.json` so every tool
call Claude Code makes appends one JSON line to a tail-able log file.

Default: **off**. Engine-enforced. Status: **beta**. Graduation target: **permanent**.

## What it does

When the user toggles the flag on in Settings > Advanced, a small panel
appears under the toggle list. The panel shows:

- The resolved `~/.claude/settings.json` path on this host.
- The events-log path Houston's hooks append to (`~/.houston/claude-hooks/events.jsonl`).
- Install status: `0` Houston-tagged hooks ⇒ not installed; non-zero ⇒ installed.
- An **Install** / **Uninstall** button that hits the engine routes.

On install, Houston writes one hook entry for each of `PreToolUse`,
`PostToolUse`, `Stop`, and `Notification`, each with matcher `*` and
the literal command:

```
mkdir -p "$(dirname '<log>')" && cat - >> '<log>' # houston-hook
```

The trailing `# houston-hook` is the tag uninstall keys on so we only
remove our own entries. Anything the user added by hand stays intact.

## Why engine-enforced

The install path is a filesystem write to a file shared with the user's
Claude Code CLI. A UI-only gate would not stop a malicious caller from
POSTing directly to `/v1/claude-hooks/install`. So the route checks the
`advanced.claude_hooks` preference engine-side and refuses with
`Forbidden { kind: "claude_hooks_disabled" }` when off.

**Uninstall stays allowed even when the flag is off** — a user who
toggled it off after install must still be able to clean up.

## Architecture

```
┌────────────────────────────────────────────┐
│ Settings > Advanced (frontend)             │
│   <FeatureGate flag="advanced.claude_hooks">│
│     <ClaudeHooksPanel />                   │
│   </FeatureGate>                           │
└────────────────────┬───────────────────────┘
                     │ tauriClaudeHooks.{status,install,uninstall}
                     ▼
┌────────────────────────────────────────────┐
│ /v1/claude-hooks/* (engine-server)         │
│   • GET  /status                           │
│   • POST /install   ← checks flag, 403s    │
│   • POST /uninstall ← always allowed       │
└────────────────────┬───────────────────────┘
                     │ houston_engine_core::claude_hooks
                     ▼
┌────────────────────────────────────────────┐
│ houston-claude-hooks (leaf crate)          │
│   • settings_path_for_home(home)           │
│   • read_status(home)  → HookStatus        │
│   • install(home)      → atomic merge      │
│   • uninstall(home)    → tagged-entry only │
└────────────────────────────────────────────┘
```

## Files touched (Phase 7 wave)

- `engine/houston-claude-hooks/` — new leaf crate (lib.rs, settings.rs, commands.rs).
- `engine/houston-engine-core/src/claude_hooks.rs` — `CoreError`-flavored facade.
- `engine/houston-engine-server/src/routes/claude_hooks.rs` — REST routes, flag-gated install.
- `Cargo.toml` — workspace member + dep entry.
- `ui/engine-client/src/client.ts` + `types.ts` — `ClaudeHookStatus`, `claudeHookStatus`, `installClaudeHooks`, `uninstallClaudeHooks`.
- `app/src/lib/tauri.ts` — `tauriClaudeHooks` facade.
- `app/src/hooks/use-claude-hooks.ts` — TanStack Query hook + mutations.
- `app/src/components/claude-hooks/claude-hooks-panel.tsx` — the panel UI.
- `app/src/components/settings/sections/advanced.tsx` — `<FeatureGate>` mount.
- `app/src/lib/featureFlags.ts` — `FLAG_REGISTRY["advanced.claude_hooks"]`.
- `app/src/locales/{en,es,pt}/claudeHooks.json` — new namespace.
- `app/src/locales/{en,es,pt}/settings.json` — `advanced.flags.claude_hooks.{label,description}`.
- `app/tests/feature-flags.test.ts` — registry shape test.

## Atomicity + safety

`houston_claude_hooks::settings::install` writes the merged JSON to a
sibling `*.houston.tmp` file and renames it onto `settings.json`. A
crash mid-write leaves the file untouched. Re-running install on top
of an existing install is a no-op — Houston-tagged entries are stripped
before re-insertion so the on-disk bytes are stable across runs.

Uninstall strips emptied containers (`"hooks": {}` and empty event
arrays) so the file's post-uninstall diff against pre-install is
clean. User-installed hooks under any of the four events ride through
unchanged.

## Known limits (v1)

- One install shape only — matcher `*` on `PreToolUse`, `PostToolUse`,
  `Stop`, `Notification`. No UI to narrow the matcher or pick a subset
  of events. Future work can add a per-event toggle.
- The hooks pipe to a JSONL file the user tails themselves. Houston
  does not (yet) tail the file and stream the events back into the
  UI — that's a future addition.
- macOS / Linux path is `$HOME/.claude/settings.json`. Windows uses
  the same path under `%USERPROFILE%`. Both match what Claude Code
  itself documents.
- The route checks the preference at call time. Toggling the flag off
  does **not** auto-uninstall — the user must click Uninstall (this
  matches the rest of the RFC's no-auto-flip rule).
