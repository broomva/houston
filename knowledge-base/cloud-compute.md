# Cloud compute plane — the sandbox control surface

How Houston runs the engine **server-side**. Greenfield; this KB tracks the
walking-skeleton spike that landed the foundation crate `houston-sandbox`.

> Relay plane vs compute plane: Houston's cloud is **two planes that scale
> oppositely**. The *relay* plane (mobile↔desktop byte broker) is shipped on
> Cloudflare Durable Objects (`houston-relay/`, see `docs/relay-operations.md`)
> — leave it. This doc is the *compute* plane: running the engine in the cloud,
> one hard-isolated sandbox per agent. Don't conflate "the relay scales on CF"
> with "Houston's compute scales" — opposite substrate problems.

## Two orthogonal axes

| Axis | Question | Houston's answer |
|---|---|---|
| **A — boundary primitive** | "how strong is the wall?" | Firecracker microVM (default), but an *implementation detail* selected at runtime |
| **B — control surface** | "who picks & configures the wall?" | one declarative `SandboxPolicy` schema → a pluggable backend registry. We own a **thin** Axis-B surface. |

The market is converging on Axis B as the developer-facing layer (MXC,
Anthropic sandbox-runtime, Kata RuntimeClass) while Axis A becomes swappable.
`houston-sandbox` is our Linux/Rust-native Axis-B surface.

## The crate: `engine/houston-sandbox`

Frontend- and engine-agnostic leaf crate. Built by reusing Houston's existing
**`ProviderAdapter` + `REGISTRY` + `Provider` newtype** pattern
(`houston-terminal-manager/src/provider/mod.rs`) — second instance of the same
registry idea, applied to the isolation backend instead of the AI provider.

```
policy.rs     SandboxPolicy { fs, net, identity, limits }   — typed enums, serde
model.rs      SandboxId · SandboxHandle · ExecRequest/Output · SnapshotId · state
error.rs      BackendError taxonomy (NotConfigured, Provision, Start, Exec, …)
runner.rs     trait SandboxBackend (the "kind", registered)  + trait SandboxRunner (live instance)
registry.rs   const BACKENDS + Backend newtype + get/all/from_str/serde + BackendConfig
backends/local/   host child process — dev rung, the benchmark floor
backends/fly/     Fly Machines (managed Firecracker) — real HTTP, mock-tested
metrics.rs    per-phase percentiles (p50/p95/p99)
bench.rs      the at-scale load driver (just another SandboxRunner consumer)
bin/sandbox-bench.rs   CLI front door
```

### Kind vs instance — why two traits

`SandboxBackend` is a stateless `&'static` singleton in `const BACKENDS`
(exactly like `ProviderAdapter`). It only knows how to `connect(&BackendConfig)
-> Arc<dyn SandboxRunner>`. The `SandboxRunner` is the *live* instance that
owns runtime state (spawned children, an HTTP client) and drives the lifecycle.
This split keeps the registry a `const` array while real backends stay stateful.

### Lifecycle

`provision → start → exec* → snapshot? → (restore) → stop`. Snapshot is the
primitive that makes scale-to-zero cheap (and speculative branching, on
backends that snapshot to an immutable artifact — `fly` suspend is in-place
resume, not yet a fork). `restore` returns the authoritative handle; its id is
backend-defined (`local` mints a new id, `fly` keeps the machine id).

| Phase | `local` backend | `fly` backend |
|---|---|---|
| provision | create workdir | create machine (`skip_launch`) |
| start | spawn process, parse `HOUSTON_ENGINE_LISTENING` banner for endpoint | start machine |
| exec | host subprocess, capture stdout/exit | machine exec endpoint |
| snapshot | copy workdir (+ `policy.json`) | **suspend** (memory snapshot to disk) |
| restore | copy back, respawn | start the suspended machine (resume) |
| stop | kill child + remove workdir | stop + destroy |

### Adding a backend

One file in `backends/<name>/` implementing `SandboxBackend` + `SandboxRunner`,
plus one line in `registry::BACKENDS`. No call site changes — same ergonomics
as adding an AI provider. (Next likely backends: `firecracker` bare jailer,
`kata` RuntimeClass — the v2 maturity rung.)

## Source of truth (files-first invariant)

The engine in its sandbox, writing its per-agent volume, stays the **only**
source of truth — isomorphic to the desktop model (engine = truth, "same code,
two doors"). The future control plane's Postgres/Redis are routing + metadata
only, never authoritative. On conflict, the volume wins. `houston-sandbox`
carries no app state for this reason.

## Credentials

`fly` reads `BackendConfig.fly_token` / `fly_app` / `fly_image`, falling back to
`$FLY_API_TOKEN` / `$FLY_APP` / `$FLY_IMAGE`. **No token → `BackendError::
NotConfigured`**, never a silent fallback (beta no-silent-failures policy).

## Dogfood + benchmark

The benchmark *is* a `SandboxRunner` consumer, so the same command produces a
`local` floor today and real Firecracker numbers from `fly` once creds land.

```bash
cargo run -p houston-sandbox --bin sandbox-bench -- --list
cargo run -p houston-sandbox --bin sandbox-bench -- --smoke          # one lifecycle, assert ok
cargo run -p houston-sandbox --bin sandbox-bench -- \
    --backend local --iterations 64 --concurrency 16 \
    --out report.json --report-md report.md
cargo run -p houston-sandbox --bin sandbox-bench -- --backend local --engine   # boot a REAL houston-engine, time its banner
cargo run -p houston-sandbox --bin sandbox-bench -- --backend fly --iterations 20  # real Firecracker (needs creds)
```

The report table maps each measured phase to the spec claim it tests
(cold-boot ≤ 1s, snap/restore p50 3–8ms, exec Δ ≤ 50ms vs baseline). **`local`
numbers are a floor** — no KVM boot, no image pull, snapshot is a dir copy. The
numbers that decide the architecture come from `--backend fly`.

### What scale dimensions the harness covers

- **Per-phase latency** p50/p95/p99 (provision/start/exec/snapshot/restore/stop)
- **Concurrency degradation** — `--concurrency N` shows how the Nth parallel
  sandbox's start latency degrades vs the 1st (observed on `local`: serial
  `start` p50 ~0.5ms → 16-way ~4ms)
- Density/cost + soak are follow-ups once `fly` is live (real VM RAM/idle cost).

## Maturity ladder (managed-first)

`v0` desktop (engine on user's Mac) → `v1` managed µVM (**Fly.io**, start here)
→ `v2` own cluster (K8s + Kata + Knative, only when scale earns the ops cost)
→ `v3` dedicated enterprise tenant VMs. The Axis-B surface makes each rung a
backend swap, not a rewrite. The full K8s north star is drawn in
`cloud-design/` (chapters C1–C6) — **deferred**, not the first step.

## Status / next (gated on go-ahead + creds)

Landed: the crate (trait + registry + `local` + `fly` + bench), 27 tests green.
Not yet built (Chunk 2+): live Fly machine run, a stateless control-plane stub
(route → wake → exec), and repointing the desktop/PWA engine-client at a
sandbox-managed engine over the same wire protocol.
