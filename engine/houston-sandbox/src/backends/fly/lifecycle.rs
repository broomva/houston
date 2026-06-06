//! The [`SandboxRunner`] lifecycle for the `fly` backend. Child module of
//! [`super`], so it reaches [`super::FlyRunner`]'s private HTTP helpers.

use super::{api, FlyRunner, BACKEND_ID};
use crate::error::BackendError;
use crate::model::{ExecOutput, ExecRequest, SandboxHandle, SandboxId, SnapshotId};
use crate::policy::{EgressMode, SandboxPolicy};
use crate::runner::SandboxRunner;
use async_trait::async_trait;

#[async_trait]
impl SandboxRunner for FlyRunner {
    async fn provision(&self, policy: &SandboxPolicy) -> Result<SandboxHandle, BackendError> {
        ensure_enforceable(policy)?;
        let req = api::build_create_request(policy, &self.image);
        let body = serde_json::to_value(&req)
            .map_err(|e| provision_err(format!("serialize create request: {e}")))?;
        let resp = self
            .post(provision_err, "", Some(body))
            .await?
            .json::<api::Machine>()
            .await
            .map_err(|e| provision_err(format!("decode machine: {e}")))?;
        self.exec_timeouts
            .lock()
            .await
            .insert(resp.id.clone(), policy.limits.exec_timeout_secs);
        Ok(SandboxHandle::provisioned(SandboxId(resp.id)))
    }

    async fn start(&self, handle: &SandboxHandle) -> Result<SandboxHandle, BackendError> {
        self.post(start_err, &format!("{}/start", handle.id.0), None)
            .await?;
        // NB: returns the private-network endpoint without an HTTP readiness
        // probe — Chunk-2 adds a /health poll like the local backend's banner.
        Ok(handle.running(Some(self.endpoint())))
    }

    async fn exec(
        &self,
        handle: &SandboxHandle,
        req: ExecRequest,
    ) -> Result<ExecOutput, BackendError> {
        if req.command.is_empty() {
            return Err(exec_err("empty exec command".into()));
        }
        let timeout = self
            .exec_timeouts
            .lock()
            .await
            .get(&handle.id.0)
            .copied()
            .filter(|t| *t > 0);
        let body = serde_json::to_value(api::ExecBody {
            cmd: req.command.join(" "),
            timeout,
        })
        .map_err(|e| exec_err(format!("serialize exec body: {e}")))?;
        let resp = self
            .post(exec_err, &format!("{}/exec", handle.id.0), Some(body))
            .await?
            .json::<api::ExecResponse>()
            .await
            .map_err(|e| exec_err(format!("decode exec response: {e}")))?;
        Ok(ExecOutput {
            exit_code: resp.exit_code,
            stdout: resp.stdout,
            stderr: resp.stderr,
        })
    }

    async fn snapshot(&self, handle: &SandboxHandle) -> Result<SnapshotId, BackendError> {
        // Suspend = memory snapshot to disk; the machine id IS the restore
        // handle. This is Fly's scale-to-zero primitive (in-place resume, not
        // a forkable artifact — see the snapshot docs in `runner.rs`).
        self.post(snapshot_err, &format!("{}/suspend", handle.id.0), None)
            .await?;
        Ok(SnapshotId(handle.id.0.clone()))
    }

    async fn restore(&self, snapshot: &SnapshotId) -> Result<SandboxHandle, BackendError> {
        // Resume the suspended machine: the restored handle keeps the SAME id
        // (the machine never went away). See the trait's `restore` contract.
        self.post(restore_err, &format!("{}/start", snapshot.0), None)
            .await?;
        let handle = SandboxHandle::provisioned(SandboxId(snapshot.0.clone()));
        Ok(handle.running(Some(self.endpoint())))
    }

    async fn stop(&self, handle: &SandboxHandle) -> Result<(), BackendError> {
        self.post(stop_err, &format!("{}/stop", handle.id.0), None)
            .await?;
        self.delete(destroy_err, &handle.id.0).await?;
        self.exec_timeouts.lock().await.remove(&handle.id.0);
        Ok(())
    }
}

/// Reject a policy whose isolation the fly backend cannot yet honor, rather
/// than silently producing a more-open machine than declared.
fn ensure_enforceable(policy: &SandboxPolicy) -> Result<(), BackendError> {
    if !policy.fs.mounts.is_empty() {
        return Err(provision_err(
            "fly backend does not yet honor fs.mounts (Chunk-2 volume wiring); remove extra mounts"
                .into(),
        ));
    }
    if policy.net.egress != EgressMode::Allow {
        return Err(provision_err(format!(
            "fly backend does not yet enforce egress restrictions (policy asked for {:?}); \
             use EgressMode::Allow until egress wiring lands",
            policy.net.egress
        )));
    }
    Ok(())
}

// One typed-error constructor per phase, so `post`/`delete` stay generic.
fn provision_err(detail: String) -> BackendError {
    BackendError::Provision {
        backend: BACKEND_ID,
        detail,
    }
}
fn start_err(detail: String) -> BackendError {
    BackendError::Start {
        backend: BACKEND_ID,
        detail,
    }
}
fn exec_err(detail: String) -> BackendError {
    BackendError::Exec {
        backend: BACKEND_ID,
        detail,
    }
}
fn snapshot_err(detail: String) -> BackendError {
    BackendError::Snapshot {
        backend: BACKEND_ID,
        detail,
    }
}
fn restore_err(detail: String) -> BackendError {
    BackendError::Restore {
        backend: BACKEND_ID,
        detail,
    }
}
fn stop_err(detail: String) -> BackendError {
    BackendError::Stop {
        backend: BACKEND_ID,
        detail,
    }
}
fn destroy_err(detail: String) -> BackendError {
    BackendError::Stop {
        backend: BACKEND_ID,
        detail: format!("destroy machine after stop: {detail}"),
    }
}
