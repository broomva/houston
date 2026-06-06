//! # houston-sandbox — the Axis-B control surface
//!
//! Houston's cloud has two planes that scale oppositely (see
//! `knowledge-base/cloud-compute.md`):
//!
//! - the **relay plane** brokers bytes between phone and desktop — already
//!   shipped on Cloudflare Durable Objects (`houston-relay/`); and
//! - the **compute plane** runs the engine server-side, one hard-isolated
//!   sandbox per agent. This crate is the compute plane's *control surface*.
//!
//! ## Two orthogonal axes
//!
//! - **Axis A — the boundary primitive** ("how strong is the wall?"):
//!   Firecracker microVM is the default, but it is an implementation detail
//!   selected at runtime.
//! - **Axis B — the control surface** ("who picks & configures the wall?"):
//!   one declarative [`SandboxPolicy`] schema feeding a pluggable backend
//!   [`registry`]. This crate owns a *thin* Axis-B surface.
//!
//! ## Reused shape
//!
//! The registry is the same pattern Houston already uses for AI providers
//! (`ProviderAdapter` + `REGISTRY` + the `Provider` newtype in
//! `houston-terminal-manager`): adding a backend is one new file plus one
//! line in [`registry::BACKENDS`]. No call site changes.
//!
//! ```ignore
//! let backend = registry::backend("local")?;          // pick Axis-A backend
//! let runner = backend.connect(&BackendConfig::default())?;
//! let handle = runner.provision(&SandboxPolicy::default()).await?;
//! let handle = runner.start(&handle).await?;           // engine now serving
//! let out = runner.exec(&handle, ExecRequest::new(["echo", "hi"])).await?;
//! let snap = runner.snapshot(&handle).await?;          // scale-to-zero primitive
//! runner.stop(&handle).await?;
//! ```
//!
//! ## What is *not* here
//!
//! No control-plane HTTP API, no auth/RBAC/billing, no engine-client
//! repoint. Those are downstream layers (control plane on Railway,
//! compute on managed Firecracker) that build *on top of* this trait.

pub mod backends;
pub mod bench;
pub mod error;
pub mod metrics;
pub mod model;
pub mod policy;
pub mod registry;
pub mod runner;

pub use error::BackendError;
pub use model::{ExecOutput, ExecRequest, SandboxHandle, SandboxId, SandboxState, SnapshotId};
pub use policy::{
    EgressMode, FsMount, FsPolicy, IdentityPolicy, MountMode, NetPolicy, ResourceLimits,
    SandboxPolicy,
};
pub use registry::{all_backends, backend, default_backend, Backend, BackendConfig};
pub use runner::{SandboxBackend, SandboxRunner};
