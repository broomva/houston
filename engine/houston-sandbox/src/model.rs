//! Lifecycle data types passed across the [`crate::runner::SandboxRunner`]
//! trait. Backend-agnostic by construction.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Opaque identifier for one provisioned sandbox. Backends map this to
/// their own primitive (a Fly machine id, a local working directory, ...).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SandboxId(pub String);

impl SandboxId {
    /// Generate a fresh random id.
    pub fn new() -> Self {
        SandboxId(Uuid::new_v4().to_string())
    }
}

impl Default for SandboxId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SandboxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifier for a sandbox snapshot — the unit that makes scale-to-zero
/// cheap (and speculative branching, on backends that snapshot to an
/// immutable artifact).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a sandbox is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxState {
    /// Allocated but not yet running.
    Provisioned,
    /// Running and (if it exposes one) serving its endpoint.
    Running,
    /// Snapshotted to zero — restorable, not consuming compute.
    Suspended,
    /// Torn down.
    Stopped,
}

/// A handle to a provisioned sandbox, returned by lifecycle calls and
/// passed back in to drive subsequent ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxHandle {
    /// This sandbox's id.
    pub id: SandboxId,
    /// Current lifecycle state.
    pub state: SandboxState,
    /// Address the in-sandbox engine serves on, once running
    /// (e.g. `http://127.0.0.1:54032`). `None` until started / for sandboxes
    /// that expose no endpoint.
    pub endpoint: Option<String>,
}

impl SandboxHandle {
    /// A freshly provisioned handle (no endpoint yet).
    pub fn provisioned(id: SandboxId) -> Self {
        Self {
            id,
            state: SandboxState::Provisioned,
            endpoint: None,
        }
    }

    /// Copy of this handle marked running, with an optional endpoint.
    pub fn running(&self, endpoint: Option<String>) -> Self {
        Self {
            id: self.id.clone(),
            state: SandboxState::Running,
            endpoint,
        }
    }
}

/// A one-shot command to run inside a sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecRequest {
    /// argv — `command[0]` is the program, the rest are arguments.
    pub command: Vec<String>,
}

impl ExecRequest {
    /// Build from any iterator of string-likes.
    pub fn new<I, S>(parts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        ExecRequest {
            command: parts.into_iter().map(Into::into).collect(),
        }
    }
}

/// Result of an [`ExecRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecOutput {
    /// Process exit code (`None` if terminated by signal).
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

impl ExecOutput {
    /// Whether the command exited successfully (code 0).
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_request_from_str_slice() {
        let r = ExecRequest::new(["echo", "hello"]);
        assert_eq!(r.command, vec!["echo", "hello"]);
    }

    #[test]
    fn handle_transitions_to_running() {
        let h = SandboxHandle::provisioned(SandboxId("abc".into()));
        assert_eq!(h.state, SandboxState::Provisioned);
        let r = h.running(Some("http://127.0.0.1:1".into()));
        assert_eq!(r.state, SandboxState::Running);
        assert_eq!(r.id, h.id);
    }

    #[test]
    fn exec_output_success() {
        let ok = ExecOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(ok.success());
    }
}
