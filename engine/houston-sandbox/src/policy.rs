//! [`SandboxPolicy`] — the declarative Axis-B schema.
//!
//! One backend-agnostic description of what a sandbox is *allowed* to do.
//! Each [`crate::runner::SandboxRunner`] translates this into its own
//! Axis-A primitive (Firecracker guest config, a Fly machine config, or a
//! local child-process environment). Domain concepts are enums, never
//! free strings, per the workspace type-safety rule.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The full isolation policy for one agent sandbox.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SandboxPolicy {
    /// Filesystem surface the sandbox sees.
    pub fs: FsPolicy,
    /// Network egress rules.
    pub net: NetPolicy,
    /// Per-agent identity / credential scoping.
    pub identity: IdentityPolicy,
    /// Resource ceilings.
    pub limits: ResourceLimits,
}

impl SandboxPolicy {
    /// A locked-down baseline: no extra mounts, egress denied, no shared
    /// identity, conservative limits. Backends widen from here.
    pub fn restricted() -> Self {
        Self {
            fs: FsPolicy::default(),
            net: NetPolicy {
                egress: EgressMode::Deny,
                allow_hosts: Vec::new(),
            },
            identity: IdentityPolicy::default(),
            limits: ResourceLimits::default(),
        }
    }
}

/// Filesystem policy: the per-agent persistent volume plus any extra mounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FsPolicy {
    /// In-sandbox path of the per-agent persistent volume (the engine's
    /// `.houston/` tree — the sole source of truth).
    pub volume_path: String,
    /// Additional mounts beyond the volume.
    pub mounts: Vec<FsMount>,
}

impl Default for FsPolicy {
    fn default() -> Self {
        Self {
            volume_path: "/vol".to_string(),
            mounts: Vec::new(),
        }
    }
}

/// One extra mount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FsMount {
    /// Path inside the sandbox.
    pub target: String,
    /// Read-only vs read-write.
    pub mode: MountMode,
}

/// Mount access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountMode {
    ReadOnly,
    ReadWrite,
}

/// Network egress policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetPolicy {
    /// Egress posture.
    pub egress: EgressMode,
    /// Hosts always reachable regardless of [`Self::egress`] (e.g. the AI
    /// provider APIs the agent must call). Honored when egress is
    /// [`EgressMode::AllowList`].
    pub allow_hosts: Vec<String>,
}

impl Default for NetPolicy {
    fn default() -> Self {
        Self {
            // Agents must reach their model provider; default to an
            // allow-list so the policy is explicit, never wide-open.
            egress: EgressMode::AllowList,
            allow_hosts: Vec::new(),
        }
    }
}

/// Egress posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressMode {
    /// No outbound network.
    Deny,
    /// Only [`NetPolicy::allow_hosts`] reachable.
    #[default]
    AllowList,
    /// Unrestricted outbound (dev only).
    Allow,
}

/// Per-agent identity / credential scoping. Tokens live with the sandbox,
/// never shared across tenants — the control plane rotates them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IdentityPolicy {
    /// Stable agent identifier (for audit + token scoping).
    pub agent_id: String,
    /// Environment variables injected into the sandbox (provider tokens,
    /// scoped per agent). Sorted for a deterministic wire shape.
    pub env: BTreeMap<String, String>,
}

/// Resource ceilings the backend enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceLimits {
    /// vCPUs.
    pub cpus: u16,
    /// Memory in MiB.
    pub memory_mb: u32,
    /// Wall-clock ceiling for a single [`crate::runner::SandboxRunner::exec`]
    /// call, in seconds. `0` means no limit.
    pub exec_timeout_secs: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpus: 1,
            memory_mb: 512,
            exec_timeout_secs: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_round_trips_through_json() {
        let p = SandboxPolicy::default();
        let json = serde_json::to_string(&p).unwrap();
        let back: SandboxPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn restricted_denies_egress() {
        assert_eq!(SandboxPolicy::restricted().net.egress, EgressMode::Deny);
    }

    #[test]
    fn enums_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&MountMode::ReadOnly).unwrap(),
            "\"read_only\""
        );
        assert_eq!(
            serde_json::to_string(&EgressMode::AllowList).unwrap(),
            "\"allow_list\""
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        let r = serde_json::from_str::<SandboxPolicy>(r#"{"bogus": 1}"#);
        assert!(r.is_err(), "deny_unknown_fields should reject extras");
    }
}
