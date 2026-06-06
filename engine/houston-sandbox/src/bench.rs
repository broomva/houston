//! The at-scale benchmark harness — *just another [`SandboxRunner`]
//! consumer*. It drives the full lifecycle through whatever backend is
//! plugged in, so the same command produces a `local` floor today and real
//! Firecracker numbers from `fly` once credentials land. The output table
//! maps each measured phase onto the claim it tests in
//! `knowledge-base/cloud-compute.md`.

use crate::error::BackendError;
use crate::metrics::{PhaseSummary, PhaseTimings};
use crate::model::ExecRequest;
use crate::policy::SandboxPolicy;
use crate::registry::{Backend, BackendConfig};
use crate::runner::SandboxRunner;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Semaphore};

/// The lifecycle phases, in driven order. Each becomes a report row.
pub const PHASES: [&str; 6] = ["provision", "start", "exec", "snapshot", "restore", "stop"];

/// One benchmark run's configuration.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub backend: Backend,
    pub backend_config: BackendConfig,
    pub policy: SandboxPolicy,
    /// Total lifecycles to run.
    pub iterations: usize,
    /// Max lifecycles in flight at once.
    pub concurrency: usize,
    /// Command timed in the `exec` phase.
    pub exec_command: Vec<String>,
}

/// The aggregated result of a benchmark run.
#[derive(Debug, Clone, Serialize)]
pub struct BenchReport {
    pub backend: String,
    pub iterations: usize,
    pub concurrency: usize,
    pub completed: usize,
    pub failures: usize,
    pub phases: Vec<PhaseSummary>,
}

/// Drive `cfg.iterations` full lifecycles (≤ `cfg.concurrency` in flight)
/// through the selected backend and aggregate per-phase latency.
pub async fn run(cfg: BenchConfig) -> Result<BenchReport, BackendError> {
    let runner = cfg.backend.connect(&cfg.backend_config)?;
    let metrics: Arc<Mutex<HashMap<&'static str, PhaseTimings>>> = Arc::new(Mutex::new(
        PHASES
            .iter()
            .map(|p| (*p, PhaseTimings::default()))
            .collect(),
    ));
    let failures = Arc::new(Mutex::new(0usize));
    let gate = Arc::new(Semaphore::new(cfg.concurrency.max(1)));

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..cfg.iterations {
        let runner = Arc::clone(&runner);
        let metrics = Arc::clone(&metrics);
        let failures = Arc::clone(&failures);
        let gate = Arc::clone(&gate);
        let policy = cfg.policy.clone();
        let exec_cmd = cfg.exec_command.clone();
        tasks.spawn(async move {
            let _permit = gate.acquire().await.expect("semaphore open");
            match one_lifecycle(runner.as_ref(), &policy, &exec_cmd).await {
                Ok(samples) => {
                    let mut m = metrics.lock().await;
                    for (phase, ms) in samples {
                        m.get_mut(phase).expect("known phase").record(ms);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "benchmark lifecycle failed");
                    *failures.lock().await += 1;
                }
            }
        });
    }
    while let Some(joined) = tasks.join_next().await {
        // A panicked task is a failed lifecycle, not a clean run — count it.
        if let Err(e) = joined {
            tracing::error!(error = %e, "benchmark task panicked");
            *failures.lock().await += 1;
        }
    }

    let metrics = Arc::try_unwrap(metrics)
        .expect("all tasks joined")
        .into_inner();
    let failures = *failures.lock().await;
    let phases = PHASES
        .iter()
        .filter_map(|p| metrics.get(*p).and_then(|t| t.summarize(p)))
        .collect();

    Ok(BenchReport {
        backend: cfg.backend.id().to_string(),
        iterations: cfg.iterations,
        concurrency: cfg.concurrency,
        completed: cfg.iterations - failures,
        failures,
        phases,
    })
}

/// One full lifecycle, returning `(phase, elapsed_ms)` samples.
async fn one_lifecycle(
    runner: &dyn SandboxRunner,
    policy: &SandboxPolicy,
    exec_cmd: &[String],
) -> Result<Vec<(&'static str, f64)>, BackendError> {
    let mut out = Vec::with_capacity(PHASES.len());

    let t = Instant::now();
    let handle = runner.provision(policy).await?;
    out.push(("provision", ms(t)));

    let t = Instant::now();
    let handle = runner.start(&handle).await?;
    out.push(("start", ms(t)));

    let t = Instant::now();
    runner
        .exec(&handle, ExecRequest::new(exec_cmd.iter().cloned()))
        .await?;
    out.push(("exec", ms(t)));

    let t = Instant::now();
    let snap = runner.snapshot(&handle).await?;
    out.push(("snapshot", ms(t)));

    let t = Instant::now();
    let restored = runner.restore(&snap).await?;
    out.push(("restore", ms(t)));

    let t = Instant::now();
    runner.stop(&handle).await?;
    out.push(("stop", ms(t)));

    // Clean up the restored sandbox (untimed — `stop` is measured once).
    runner.stop(&restored).await?;

    Ok(out)
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

impl BenchReport {
    /// Render the report as a markdown table mapping each measured phase to
    /// the spec claim it tests.
    pub fn to_markdown(&self) -> String {
        let mut s = format!(
            "## Sandbox benchmark — backend `{}`\n\n\
             - iterations: {} (concurrency {})\n- completed: {} · failures: {}\n\n\
             | phase | claim it tests | p50 ms | p95 ms | p99 ms | max ms |\n\
             |---|---|---|---|---|---|\n",
            self.backend, self.iterations, self.concurrency, self.completed, self.failures
        );
        for p in &self.phases {
            s.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                p.phase,
                claim_for(&p.phase),
                p.p50_ms,
                p.p95_ms,
                p.p99_ms,
                p.max_ms
            ));
        }
        if self.backend == "local" {
            s.push_str(
                "\n> `local` numbers are a floor: no KVM boot, no image pull, snapshot is a \
                 directory copy. Substrate claims are decided by `--backend fly`.\n",
            );
        }
        s
    }
}

/// The spec claim each phase's latency validates.
fn claim_for(phase: &str) -> &'static str {
    match phase {
        "start" => "cold-boot ≤ 1s end-to-end (C3 gate)",
        "snapshot" => "snap/restore p50 3–8ms (scale-to-zero primitive)",
        "restore" => "restore ≪ cold start (scale-to-zero economics)",
        "exec" => "isolation adds ≤ 50ms vs baseline (C3 gate)",
        "provision" => "create + volume attach budget",
        "stop" => "teardown + volume detach (PV tension, risk §09)",
        _ => "",
    }
}
