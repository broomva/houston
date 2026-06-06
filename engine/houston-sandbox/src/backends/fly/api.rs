//! Fly Machines API wire types + the policy→machine translation. Kept
//! separate from the HTTP client so request shaping is unit-testable
//! without a network (the part we *can* verify deterministically; live Fly
//! correctness is the Chunk-2 gate).
//!
//! Field names track the canonical `superfly/fly-go` types: the exec body is
//! `cmd` (string), and guest sizing is `cpus` / `memory_mb` / `cpu_kind`.

use crate::policy::SandboxPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Body of `POST /apps/{app}/machines`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CreateMachineRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub config: MachineConfig,
    /// Create without booting — we boot explicitly in `start`, so cold-boot
    /// latency is attributed to the right lifecycle phase in the benchmark.
    pub skip_launch: bool,
}

/// A machine's runtime config.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MachineConfig {
    pub image: String,
    pub guest: Guest,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Reap the machine when it stops, so a crashed sandbox leaves nothing
    /// billing behind.
    pub auto_destroy: bool,
}

/// Guest sizing — the Axis-A resource shape derived from the policy.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Guest {
    pub cpus: u16,
    pub memory_mb: u32,
    pub cpu_kind: CpuKind,
}

/// Fly's CPU class. An enum, not a free string, per the workspace
/// type-safety rule. Chunk 1 only provisions shared guests; add a
/// `Performance` variant here when we wire the performance tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuKind {
    Shared,
}

/// Subset of the machine object Fly returns. Only `id` is consumed today;
/// the rest of the object is ignored by serde.
#[derive(Debug, Clone, Deserialize)]
pub struct Machine {
    pub id: String,
}

/// Body of `POST /apps/{app}/machines/{id}/exec`. Matches `fly-go`'s
/// `MachineExecRequest`: `cmd` is a single string.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecBody {
    pub cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Response from the exec endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecResponse {
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
}

/// Translate a backend-agnostic [`SandboxPolicy`] into a Fly create request.
/// This is the Axis-B → Axis-A mapping: limits → guest sizing, identity env
/// → machine env, volume path → engine home, plus the standalone-engine
/// wiring (`HOUSTON_NO_PARENT_WATCHDOG`).
///
/// Network egress and extra mounts are validated by the caller before this
/// runs (see `fly::lifecycle`) — they are NOT silently dropped here.
pub fn build_create_request(policy: &SandboxPolicy, image: &str) -> CreateMachineRequest {
    let mut env: BTreeMap<String, String> = policy.identity.env.clone();
    env.insert("HOUSTON_HOME".into(), policy.fs.volume_path.clone());
    env.insert("HOUSTON_BIND".into(), "0.0.0.0:8080".into());
    env.insert("HOUSTON_BIND_ALL".into(), "1".into());
    env.insert("HOUSTON_NO_PARENT_WATCHDOG".into(), "1".into());

    CreateMachineRequest {
        region: None,
        config: MachineConfig {
            image: image.to_string(),
            guest: Guest {
                cpus: policy.limits.cpus.max(1),
                memory_mb: policy.limits.memory_mb.max(256),
                cpu_kind: CpuKind::Shared,
            },
            env,
            auto_destroy: true,
        },
        skip_launch: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_limits_to_guest() {
        let mut policy = SandboxPolicy::default();
        policy.limits.cpus = 4;
        policy.limits.memory_mb = 2048;
        let req = build_create_request(&policy, "registry.fly.io/houston:latest");
        assert_eq!(req.config.guest.cpus, 4);
        assert_eq!(req.config.guest.memory_mb, 2048);
        assert!(req.skip_launch);
        assert!(req.config.auto_destroy);
    }

    #[test]
    fn injects_standalone_engine_wiring() {
        let req = build_create_request(&SandboxPolicy::default(), "img");
        assert_eq!(
            req.config.env.get("HOUSTON_NO_PARENT_WATCHDOG").unwrap(),
            "1"
        );
        assert_eq!(req.config.env.get("HOUSTON_BIND").unwrap(), "0.0.0.0:8080");
    }

    #[test]
    fn clamps_tiny_guests_to_a_floor() {
        let policy = SandboxPolicy {
            limits: crate::policy::ResourceLimits {
                cpus: 0,
                memory_mb: 0,
                exec_timeout_secs: 0,
            },
            ..Default::default()
        };
        let req = build_create_request(&policy, "img");
        assert_eq!(req.config.guest.cpus, 1);
        assert_eq!(req.config.guest.memory_mb, 256);
    }

    #[test]
    fn serializes_to_expected_json_shape() {
        let req = build_create_request(&SandboxPolicy::default(), "img");
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["skip_launch"], true);
        assert_eq!(v["config"]["image"], "img");
        assert_eq!(v["config"]["guest"]["cpu_kind"], "shared");
        assert!(v.get("region").is_none(), "region omitted when None");
    }

    #[test]
    fn exec_body_uses_cmd_string_not_command_array() {
        let body = ExecBody {
            cmd: "echo hi".into(),
            timeout: Some(30),
        };
        let v = serde_json::to_value(&body).unwrap();
        assert_eq!(v["cmd"], "echo hi");
        assert_eq!(v["timeout"], 30);
        assert!(v.get("command").is_none(), "must be `cmd`, not `command`");
    }
}
