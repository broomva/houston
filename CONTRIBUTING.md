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

---

## Technical Stack & Quick Start

Houston is a monorepo structured with high separation of concerns between TypeScript/React frontends and a standalone Rust backend.

### Prerequisites

- **Bun** or **Node.js** (v22+)
- **Rust Toolchain** (Stable)
- **Tauri CLI** (for running the desktop wrapper)

### Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/gethouston/houston.git
cd houston
# Install dependencies (automatically initializes Husky pre-commit hooks)
bun install

# Run type-checking & Rust cargo checks
make verify-all
```

---

## Monorepo Architecture & Package Boundaries

All code in Houston lives within strict logical domains to ensure the engine remains highly reusable and the UI stays modular:

1. **`engine/` (Rust Workspace Crates)**: Standalone, frontend-agnostic execution runtime. No Tauri dependencies, no React assumptions. Speaks exclusively HTTP and WebSocket.
2. **`ui/` (React Workspace Packages)**: Generic, props-driven, UI-only `@houston-ai/*` packages. No global state stores (e.g. Zustand, Redux) are allowed inside libraries. Use props over stores.
3. **`app/` (Tauri Desktop App)**: Orchestrates the UI packages and spawns the Rust engine as a sidecar process. Features target-specific adapters linking Tauri endpoints to the Engine protocol.
4. **`mobile/` (React PWA)**: Serves mobile viewport components dynamically.

Other top-level directories: `desktop-mobile-bridge/` (Cloudflare Worker pairing App + Mobile), `store/` (agent registry), `website/` (gethouston.ai landing), `always-on/` / `teams/` / `cloud/` (future hosted products, placeholders).

---

## Strict Coding & UX Standards

We enforce high-fidelity design aesthetics and defensive programming practices. Pull requests violating these rules will be rejected:

### 1. File Size Constraints & Organization
- **Maximum 200 lines per file** for all JavaScript, TypeScript, and Rust source files (excluding unit tests). If a file exceeds this length, extract modules.
- **Maximum 500 lines for CSS files**. Keep layout systems clean and modular.
- **No `@/` path aliases in `ui/` packages**. Relative imports within a package; package-name imports between.

### 2. Banned UX and UI Anti-Patterns
- **No hover-only affordances**: All interactive elements (buttons, edit triggers, delete controls) MUST be fully visible without hovering. Hover effects should enhance, never gate.
- **Rich Aesthetics & Color Harmonies**: Use premium gradients, glassmorphism, dynamic transitions, and modern typography. Never use generic or raw CSS colors.
- **Props over stores in `ui/` packages**: no Zustand/Redux imports inside libraries.

### 3. Beta-Stage Error Surfacing (No Silent Failures)
- **Never swallow errors**: Every user-initiated action that fails MUST bubble up to the user as a visible toast with a "Report Bug" affordance.
- Banned Rust patterns: `let _ = <fallible>`, `.ok()`, `.unwrap_or_default()`, `let _ = <fallible>.await` on user operations.
- Banned TypeScript patterns: Empty `.catch(() => {})`, swallow `try-catch` blocks, or generic "An error occurred" toasts. Use `errorMessage(err)` for verbose surfacing.

### 4. Universal Internationalization (i18n)
- Houston ships in English (`en`), Spanish (`es`), and Portuguese (`pt`).
- **No literal English text in JSX**: All strings, labels, and aria-attributes must run through the `t()` function.
- Spanish = Latin-American neutral. Portuguese = Brazilian.
- Running `bun --filter houston-app check-locales` validates layout parity and blocks em-dashes (`—`) in copy.

---

## Local Git Verification (Husky & lint-staged)

We utilize machine-enforced git guardrails to prevent broken code, compilation errors, or version sync drifts from reaching origin.

When you run `git commit`, the Husky pre-commit hook automatically invokes `lint-staged` to execute:
- **TypeScript Workspace Typecheck**: runs `bun run typecheck` (verifies entire workspace type safety).
- **Translation Parity Audit**: runs `check-locales` (verifies JSON structure matching).
- **Cargo Format Checks**: runs `rustfmt --check` on modified `.rs` files.
- **Workspace Release Sync**: runs `./scripts/cargo-sync-check.sh` on changes to `package.json` to verify that no version discrepancies exist between npm and cargo workspaces.

If any check fails, your commit will be blocked. To run checks manually:

```bash
# Verify typecheck, locales, cargo-sync-check, and Rust tests
make verify-all

# Or run individual commands manually:
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

---

## Conventional Commits

We strictly follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. Commits must be formatted as:

```
<type>(<scope>): <short description>

[optional body]
```

### Approved Types:
- `feat`: A new user-facing feature.
- `fix`: A bug fix.
- `docs`: Documentation-only changes.
- `style`: Formatting, missing semi-colons, no code changes.
- `refactor`: A code change that neither fixes a bug nor adds a feature.
- `test`: Adding missing tests or correcting existing tests.
- `chore`: Updating build scripts, release workflows, or package dependencies.

---

## Pull Request & Issue Sync Policy

### GitHub-to-Linear Synchronization

Our core planning, issue tracking, and roadmap management are handled internally using **Linear**.
- When a public issue is created on GitHub, our sync bots automatically mirror the report onto our internal Linear board.
- When an engineer claims the task or links a branch, status updates flow back to the GitHub issue transparently.
- When formatting an issue, please use our YAML Bug Report or Feature Request forms to guarantee our automation syncs it under the right priority tag.

## Dogfooding Houston (validating from the client POV)

Before merging substantive feature PRs, exercise the change against the running
app — not just against `cargo check` and `bun run typecheck`. Houston is a Tauri +
sidecar app, so the canonical interaction surfaces are hybrid (engine API,
WebView via `cliclick`, real Chrome at `:1420` via Interceptor for the vite
frontend). Full pattern + surfaces matrix + canonical arc + gotchas at:

- **`docs/development/dogfood-pattern.html`** — the canonical Houston dogfood
  loop, captured from a real PR (the Tauri + sidecar worked example)

The shape of a Dogfood Plan to include in your PR body (or `docs/dogfood-plan.md` if you prefer a file):

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

## Monorepo Contribution Pipeline

1. Confirm your change fits the bar in [Before you open a PR](#before-you-open-a-pr).
2. Find an open issue or submit a YAML proposal in GitHub.
3. Fork the repository and create an isolated branch from `main`: `git checkout -b feat/your-feature-name`.
4. Implement your changes keeping files <200 lines, fully type-safe, and i18n-compliant.
5. Verify your work using `make verify-all`.
6. Push your branch and open a Pull Request targeting `main`.
7. Ensure the automated GitHub Actions CI/CD check passes completely.
