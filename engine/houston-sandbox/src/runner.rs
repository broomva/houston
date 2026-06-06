//! The two traits at the heart of the Axis-B control surface.
//!
//! [`SandboxBackend`] is the *kind* — a stateless, registered singleton
//! (Firecracker, Fly, local), held as `&'static dyn` in
//! [`crate::registry::BACKENDS`], exactly like Houston's `ProviderAdapter`.
//! [`SandboxRunner`] is the *live instance* it builds — it owns runtime
//! state (spawned processes, an HTTP client) and drives the lifecycle.
//!
//! Splitting kind from instance is what lets the registry stay a `const`
//! array of zero-state singletons while real backends keep mutable state.

use crate::error::BackendError;
use crate::model::{ExecOutput, ExecRequest, SandboxHandle, SnapshotId};
use crate::policy::SandboxPolicy;
use crate::registry::BackendConfig;
use async_trait::async_trait;
use std::sync::Arc;

/// A registered isolation backend (the Axis-A primitive selector).
///
/// Stateless and cheap to reference repeatedly — the registry hands out
/// shared `&'static` singletons. All runtime state lives on the
/// [`SandboxRunner`] that [`Self::connect`] builds.
pub trait SandboxBackend: Send + Sync + 'static {
    /// Stable lower-snake-case id used in config, JSON, and URLs
    /// (`"local"`, `"fly"`, `"firecracker"`).
    fn id(&self) -> &'static str;

    /// Aliases accepted by [`crate::registry::backend`] in addition to
    /// [`Self::id`].
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// One-line human description, surfaced in `sandbox-bench --list`.
    fn description(&self) -> &'static str;

    /// Build a live runner from runtime configuration.
    ///
    /// Returns [`BackendError::NotConfigured`] when required credentials are
    /// absent — the caller surfaces that rather than silently degrading.
    fn connect(&self, config: &BackendConfig) -> Result<Arc<dyn SandboxRunner>, BackendError>;
}

/// A live sandbox controller. One instance manages many sandboxes keyed by
/// [`crate::model::SandboxId`]; lifecycle methods take `&self` and use
/// interior mutability so the runner can be shared across concurrent tasks
/// (the benchmark harness drives N in parallel through one runner).
///
/// Lifecycle: `provision → start → exec* → snapshot? → (restore) → stop`.
#[async_trait]
pub trait SandboxRunner: Send + Sync {
    /// Allocate a sandbox per the policy. Does not start it.
    async fn provision(&self, policy: &SandboxPolicy) -> Result<SandboxHandle, BackendError>;

    /// Boot the engine inside a provisioned sandbox and wait until it is
    /// ready. Returns an updated handle carrying the serving endpoint.
    async fn start(&self, handle: &SandboxHandle) -> Result<SandboxHandle, BackendError>;

    /// Run a one-shot command inside a running sandbox.
    async fn exec(
        &self,
        handle: &SandboxHandle,
        req: ExecRequest,
    ) -> Result<ExecOutput, BackendError>;

    /// Snapshot a sandbox — the primitive that makes **scale-to-zero** cheap
    /// (and, on backends that snapshot to an immutable artifact, speculative
    /// branching). The sandbox may keep running or be suspended depending on
    /// the backend: `local` copies the volume (the sandbox keeps running),
    /// `fly` suspends the machine (in-place memory snapshot, resumed by
    /// `restore` — not yet a forkable artifact).
    async fn snapshot(&self, handle: &SandboxHandle) -> Result<SnapshotId, BackendError>;

    /// Recreate a sandbox from a snapshot. Returns a **started** handle, which
    /// is the authoritative handle to use afterwards. Its [`SandboxHandle::id`]
    /// is backend-defined: `local` mints a fresh id (copy → new sandbox),
    /// `fly` keeps the same machine id (resume). Callers must use the returned
    /// handle and not assume it equals (or differs from) any prior id.
    async fn restore(&self, snapshot: &SnapshotId) -> Result<SandboxHandle, BackendError>;

    /// Tear the sandbox down and release its resources.
    async fn stop(&self, handle: &SandboxHandle) -> Result<(), BackendError>;
}
