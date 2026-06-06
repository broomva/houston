//! Concrete isolation backends. Each is one [`crate::runner::SandboxBackend`]
//! singleton plus the [`crate::runner::SandboxRunner`] it builds.
//!
//! - [`local`] — a child process on the host, no isolation. The dev / v1
//!   rung of the maturity ladder; the floor for benchmark numbers.
//! - [`fly`] — Fly Machines (managed Firecracker), the lean v1 cloud host.

pub mod fly;
pub mod local;
pub mod proc;
