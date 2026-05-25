# Advanced slash skills

`advanced.slash_skills` exposes a `/` picker in the chat composer for users
who already maintain skill folders through Claude Code or other agent tools.

## Enforcement

- Surface: UI flag.
- Default: off.
- Registry key: `advanced.slash_skills`.
- Locale keys: `settings:advanced.flags.slash_skills.*`.

When disabled, the composer behaves exactly as before and only the existing
Skills button/picker is visible. When enabled, typing `/` in the composer opens
a keyboard-navigable menu.

## Sources

The engine route is `GET /v1/skills/catalog`. It always includes the active
agent's `.agents/skills` directory. With `includeExternal=true`, it also scans
the parent workspace, project `.claude/skills`, `~/.claude/skills`, and
`~/.agents/skills`.

External sources are read-only. The scan parses directory-shaped
`*/SKILL.md` skills and intentionally avoids Houston's flat-file migration,
because the picker must not rewrite user-owned Claude or agent folders.

## Invocation

Selecting an agent-local Houston skill uses the existing Skill pipeline.
Selecting an external skill pins the same selected-skill chip and persists the
same skill-invocation marker, but the hidden prompt sent to the downstream CLI
includes the external `SKILL.md` path plus source label. The user sees the card,
not the path.
