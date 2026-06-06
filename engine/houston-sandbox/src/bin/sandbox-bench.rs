//! `sandbox-bench` — drive the sandbox lifecycle through any registered
//! backend and print per-phase latency. Backend-agnostic: `--backend local`
//! gives the dev-box floor today; `--backend fly` gives real Firecracker
//! numbers once `$FLY_API_TOKEN` / `$FLY_APP` / `$FLY_IMAGE` are set.
//!
//! ```text
//! sandbox-bench --backend local --iterations 50 --concurrency 10
//! sandbox-bench --backend local --engine            # time a real houston-engine boot
//! sandbox-bench --backend fly --iterations 20       # real Firecracker (needs creds)
//! sandbox-bench --list
//! ```

use houston_sandbox::bench::{run, BenchConfig};
use houston_sandbox::{all_backends, backend, BackendConfig, SandboxPolicy};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match real_main().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sandbox-bench: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn real_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }
    if args.iter().any(|a| a == "--list") {
        for b in all_backends() {
            println!("{:<12} {}", b.id(), b.description());
        }
        return Ok(());
    }

    let opts = Opts::parse(&args)?;
    let backend = backend(&opts.backend).map_err(|e| e.to_string())?;

    // `local`: default to a portable trivial process (the floor). `--engine`
    // boots a real houston-engine and times its readiness banner.
    let backend_config = if opts.engine {
        BackendConfig {
            launch_command: Some(vec!["houston-engine".into()]),
            ..Default::default()
        }
    } else {
        BackendConfig {
            launch_command: Some(opts.launch.clone()),
            ready_marker: Some(String::new()), // spawn = ready (no banner)
            ..Default::default()
        }
    };

    let cfg = BenchConfig {
        backend,
        backend_config,
        policy: SandboxPolicy::default(),
        iterations: if opts.smoke { 1 } else { opts.iterations },
        concurrency: if opts.smoke { 1 } else { opts.concurrency },
        exec_command: opts.exec.clone(),
    };

    let report = run(cfg).await.map_err(|e| e.to_string())?;

    if opts.smoke {
        if report.failures > 0 {
            return Err(format!(
                "smoke FAILED: {} lifecycle error(s)",
                report.failures
            ));
        }
        println!("smoke PASS — full lifecycle ok on `{}`", report.backend);
    }

    let md = report.to_markdown();
    println!("{md}");

    if let Some(path) = &opts.out {
        let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| format!("write {path}: {e}"))?;
        eprintln!("wrote JSON → {path}");
    }
    if let Some(path) = &opts.report_md {
        std::fs::write(path, &md).map_err(|e| format!("write {path}: {e}"))?;
        eprintln!("wrote markdown → {path}");
    }
    Ok(())
}

struct Opts {
    backend: String,
    iterations: usize,
    concurrency: usize,
    smoke: bool,
    engine: bool,
    launch: Vec<String>,
    exec: Vec<String>,
    out: Option<String>,
    report_md: Option<String>,
}

impl Opts {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut o = Opts {
            backend: "local".into(),
            iterations: 10,
            concurrency: 1,
            smoke: false,
            engine: false,
            launch: vec!["sleep".into(), "30".into()],
            exec: vec!["true".into()],
            out: None,
            report_md: None,
        };
        let mut i = 0;
        while i < args.len() {
            let a = args[i].clone();
            match a.as_str() {
                "--backend" => o.backend = take(args, &mut i)?,
                "--iterations" => {
                    o.iterations = take(args, &mut i)?
                        .parse()
                        .map_err(|_| "bad --iterations")?
                }
                "--concurrency" => {
                    o.concurrency = take(args, &mut i)?
                        .parse()
                        .map_err(|_| "bad --concurrency")?
                }
                "--launch" => {
                    o.launch = take(args, &mut i)?
                        .split_whitespace()
                        .map(String::from)
                        .collect()
                }
                "--exec" => {
                    o.exec = take(args, &mut i)?
                        .split_whitespace()
                        .map(String::from)
                        .collect()
                }
                "--out" => o.out = Some(take(args, &mut i)?),
                "--report-md" => o.report_md = Some(take(args, &mut i)?),
                "--smoke" => o.smoke = true,
                "--engine" => o.engine = true,
                other => return Err(format!("unknown flag `{other}` (try --help)")),
            }
            i += 1;
        }
        Ok(o)
    }
}

/// Consume the value following a flag at `args[*i]`, advancing the cursor.
fn take(args: &[String], i: &mut usize) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| format!("{} needs a value", args[*i - 1]))
}

fn print_help() {
    println!(
        "sandbox-bench — drive the sandbox lifecycle and report per-phase latency\n\n\
         FLAGS\n\
         \x20 --backend <id>       backend to drive (default: local; see --list)\n\
         \x20 --iterations <n>     total lifecycles (default: 10)\n\
         \x20 --concurrency <n>    max in flight at once (default: 1)\n\
         \x20 --engine             local: boot a real houston-engine, time its banner\n\
         \x20 --launch \"<cmd>\"     local: process to spawn (default: \"sleep 30\")\n\
         \x20 --exec \"<cmd>\"       command timed in the exec phase (default: \"true\")\n\
         \x20 --out <path>         write JSON report\n\
         \x20 --report-md <path>   write markdown report\n\
         \x20 --smoke              run one lifecycle, assert it succeeds\n\
         \x20 --list               list registered backends\n"
    );
}
