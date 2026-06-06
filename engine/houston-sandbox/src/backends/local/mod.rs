//! `local` backend — runs the engine as a host child process with no
//! isolation. This is the dev / v1 rung of the maturity ladder: it
//! exercises the whole [`SandboxRunner`] lifecycle for real (real spawn,
//! real exec, real file-copy snapshot) on a machine with no cloud account.
//!
//! Honest limits: no kernel boundary, and snapshot is a directory copy, not
//! a Firecracker memory snapshot — so its cold-start / density numbers are a
//! *floor*, not the substrate's real behavior. The numbers that decide the
//! architecture come from the `fly` backend on real hardware.

mod lifecycle;

use crate::error::BackendError;
use crate::model::{SandboxId, SnapshotId};
use crate::policy::SandboxPolicy;
use crate::registry::BackendConfig;
use crate::runner::{SandboxBackend, SandboxRunner};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

pub(crate) const BACKEND_ID: &str = "local";

/// The registered `local` backend singleton.
pub struct LocalBackend;

/// Singleton instance held in the registry.
pub static LOCAL: LocalBackend = LocalBackend;

impl SandboxBackend for LocalBackend {
    fn id(&self) -> &'static str {
        BACKEND_ID
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["host", "process"]
    }
    fn description(&self) -> &'static str {
        "host child process, no isolation (dev rung; benchmark floor)"
    }
    fn connect(&self, config: &BackendConfig) -> Result<Arc<dyn SandboxRunner>, BackendError> {
        let base = std::env::temp_dir().join("houston-sandbox");
        Ok(Arc::new(LocalRunner {
            base,
            launch_command: config
                .launch_command
                .clone()
                .unwrap_or_else(|| vec!["houston-engine".to_string()]),
            ready_marker: config
                .ready_marker
                .clone()
                .unwrap_or_else(|| "HOUSTON_ENGINE_LISTENING".to_string()),
            sandboxes: Mutex::new(HashMap::new()),
        }))
    }
}

/// One live host-process sandbox.
struct LiveSandbox {
    workdir: PathBuf,
    policy: SandboxPolicy,
    child: Option<Child>,
}

/// Live controller for host-process sandboxes. Lifecycle impl lives in
/// [`lifecycle`].
pub struct LocalRunner {
    base: PathBuf,
    launch_command: Vec<String>,
    ready_marker: String,
    sandboxes: Mutex<HashMap<SandboxId, LiveSandbox>>,
}

impl LocalRunner {
    fn sb_dir(&self, id: &SandboxId) -> PathBuf {
        self.base.join("sb").join(&id.0)
    }
    fn snap_dir(&self, id: &SnapshotId) -> PathBuf {
        self.base.join("snap").join(&id.0)
    }

    /// Env the host child gets: engine wiring plus the policy's scoped vars.
    fn env_for(&self, workdir: &Path, policy: &SandboxPolicy) -> Vec<(String, String)> {
        let mut env = vec![
            ("HOUSTON_HOME".into(), workdir.display().to_string()),
            ("HOUSTON_BIND".into(), "127.0.0.1:0".into()),
            ("HOUSTON_NO_PARENT_WATCHDOG".into(), "1".into()),
        ];
        env.extend(policy.identity.env.iter().map(|(k, v)| (k.clone(), v.clone())));
        env
    }
}
