//! [`BackendError`] — the typed failure taxonomy for the control surface.
//!
//! Every fallible lifecycle op returns a typed variant so callers (and the
//! control plane above) can surface the real reason — never a swallowed
//! `Result`, per the workspace no-silent-failures rule.

use thiserror::Error;

/// A sandbox backend failure.
#[derive(Debug, Error)]
pub enum BackendError {
    /// The backend needs runtime configuration that is missing (e.g. a Fly
    /// API token). Distinct from a transport failure: nothing was attempted.
    #[error("backend `{backend}` is not configured: {detail}")]
    NotConfigured {
        backend: &'static str,
        detail: String,
    },

    /// Provisioning the sandbox failed (create machine / allocate volume).
    #[error("provision failed on `{backend}`: {detail}")]
    Provision {
        backend: &'static str,
        detail: String,
    },

    /// Starting an already-provisioned sandbox failed (boot / readiness).
    #[error("start failed on `{backend}`: {detail}")]
    Start {
        backend: &'static str,
        detail: String,
    },

    /// An exec inside the sandbox failed to launch or run.
    #[error("exec failed on `{backend}`: {detail}")]
    Exec {
        backend: &'static str,
        detail: String,
    },

    /// Snapshot (the scale-to-zero primitive) failed.
    #[error("snapshot failed on `{backend}`: {detail}")]
    Snapshot {
        backend: &'static str,
        detail: String,
    },

    /// Restore from a snapshot failed.
    #[error("restore failed on `{backend}`: {detail}")]
    Restore {
        backend: &'static str,
        detail: String,
    },

    /// Stopping / tearing down the sandbox failed.
    #[error("stop failed on `{backend}`: {detail}")]
    Stop {
        backend: &'static str,
        detail: String,
    },

    /// The caller referenced a sandbox id this runner does not know.
    #[error("unknown sandbox `{0}`")]
    UnknownSandbox(String),

    /// An operation exceeded its deadline.
    #[error("`{op}` on `{backend}` timed out after {secs}s")]
    Timeout {
        backend: &'static str,
        op: &'static str,
        secs: u64,
    },

    /// The control surface was asked for a backend id that is not registered.
    #[error("unknown sandbox backend `{0}`")]
    UnknownBackend(String),
}

impl BackendError {
    /// The backend id this error originated from, when known.
    pub fn backend(&self) -> Option<&'static str> {
        match self {
            BackendError::NotConfigured { backend, .. }
            | BackendError::Provision { backend, .. }
            | BackendError::Start { backend, .. }
            | BackendError::Exec { backend, .. }
            | BackendError::Snapshot { backend, .. }
            | BackendError::Restore { backend, .. }
            | BackendError::Stop { backend, .. }
            | BackendError::Timeout { backend, .. } => Some(backend),
            BackendError::UnknownSandbox(_) | BackendError::UnknownBackend(_) => None,
        }
    }
}
