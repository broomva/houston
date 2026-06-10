# Contributing to Houston

Houston is a small team shipping fast. We're glad you want to help, and we have a posted bar so review stays sustainable.

## Before you open a PR

Read this section. If your PR doesn't fit, we'll close it without arguing.

1. **Open an issue first** for anything that isn't a bug fix under ~50 LOC. No surprise PRs for refactors, new tooling, governance, or docs about how to contribute. If we haven't agreed it's worth building, don't build it.
2. **One open PR at a time** per contributor. Open the next one only after the previous merges or closes.
3. **Scratch your own itch.** The PR must fix a bug you hit or add a feature you actually use in Houston. "I thought the repo could use X" isn't a reason. Speculative improvements are work for us, not value.
4. **AI-generated is fine, you reviewing the diff isn't optional.** Use Claude Code, Cursor, whatever. But you read the diff first. If the PR body has to justify why to ship it ("not really an upgrade but…"), don't ship it. PRs that read as raw autonomous-loop output get closed.
5. **No importing external frameworks.** Don't add your own methodology, RFCs, or doctrine to Houston's `knowledge-base/` or `docs/`. Link from your repo to ours, not the other way around.
6. **Stacked PRs get one shot.** If the base PR doesn't land, the stack is dead. Don't chain four deep.

If you're unsure whether something fits, open an issue and ask. Cheaper than a closed PR for both of us.

## Getting Started

```bash
git clone https://github.com/gethouston/houston.git
cd houston
bun install
cargo check --workspace
```

## Development

```bash
# Run the Houston app
cd app && bun run tauri dev

# Run the showcase
cd showcase && bun run dev

# TypeScript check
bun run typecheck

# Rust check
cargo check --workspace

# Rust tests
cargo test --workspace
```

## Structure

- `ui/` — React packages (@houston-ai/*)
- `engine/` — Rust crates (houston-*) — frontend-agnostic backend
- `app/` — Houston App (Tauri desktop)
- `mobile/` — Houston Mobile companion
- `desktop-mobile-bridge/` — Cloudflare Worker pairing App + Mobile
- `store/` — Houston Store (agent registry)
- `website/` — gethouston.ai landing
- `always-on/` · `teams/` · `cloud/` — future hosted products (placeholders)

## Pull Requests

1. Confirm your change fits the bar in [Before you open a PR](#before-you-open-a-pr)
2. Create a feature branch from `main`
3. Make your changes
4. Run `bun run typecheck` and `cargo check --workspace`
5. Open a PR to `main`, fill out the template honestly

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — New feature
- `fix:` — Bug fix
- `docs:` — Documentation
- `chore:` — Maintenance
- `refactor:` — Code restructuring

## Code Style

- 200 line file limit (excluding tests)
- No hover-only affordances
- Props over stores in library packages
- No `@/` path aliases in packages

## Dogfooding Houston (validating from the client POV)

Before merging substantive feature PRs, exercise the change against the running
app — not just against `cargo check` and `pnpm typecheck`. Houston is a Tauri +
sidecar app, so the canonical interaction surfaces are hybrid (engine API,
WebView via `cliclick`, real Chrome at `:1420` via Interceptor for the vite
frontend). Full pattern + surfaces matrix + canonical arc + gotchas at:

- **`docs/development/dogfood-pattern.html`** — the canonical Houston dogfood
  loop, captured from a real PR (the Tauri + sidecar worked example)

The shape of a Dogfood Plan to include in your PR body (or `docs/dogfood-
plan.md` if you prefer a file):

```markdown
**Dogfood Plan** (stack: tauri-sidecar)

- **Entry surface**: <route, window, CLI command, or API endpoint changed>
- **Driver**: <engine API curl, cliclick coords, Interceptor at :1420, screencapture>
- **Evidence**: <screenshot path, response body, log line, recording>
- **Smoke**: <one-line "didn't obviously break" check>
- **End-to-end**: <multi-step user flow the change is supposed to support>
- **Receipt anchor**: <PR comment, file, or message-id where evidence lives>
```

The Dogfood Plan is the *upstream* of the Dogfood Receipt (the evidence
table you produce before claiming the work complete). Reasoning isn't
validation; interaction is.

Related: this pattern composes with the [bstack P11 cookbook](https://github.com/broomva/bstack/blob/main/references/dogfood-patterns.md)
which generalizes the same shape across other stacks (Next.js, Expo RN, Rust
CLI, REST API, MCP server). RFC tracking: [#243](https://github.com/gethouston/houston/issues/243).
