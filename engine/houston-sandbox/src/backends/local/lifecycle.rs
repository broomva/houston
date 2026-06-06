//! The [`SandboxRunner`] lifecycle for the `local` backend. Child module of
//! [`super`], so it reaches the private [`super::LocalRunner`] and
//! [`super::LiveSandbox`] internals directly.
//!
//! Lock discipline: the global `sandboxes` Mutex is held only to read/insert
//! map entries, never across a process spawn or a disk copy — otherwise one
//! slow boot would serialize every concurrent lifecycle and the
//! `--concurrency N` benchmark would measure nothing.

use super::{LiveSandbox, LocalRunner, BACKEND_ID};
use crate::backends::proc::{copy_tree, spawn_ready};
use crate::error::BackendError;
use crate::model::{ExecOutput, ExecRequest, SandboxHandle, SandboxId, SnapshotId};
use crate::policy::SandboxPolicy;
use crate::runner::SandboxRunner;
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
impl SandboxRunner for LocalRunner {
    async fn provision(&self, policy: &SandboxPolicy) -> Result<SandboxHandle, BackendError> {
        let id = SandboxId::new();
        let workdir = self.sb_dir(&id);
        tokio::fs::create_dir_all(&workdir)
            .await
            .map_err(|e| provision_err(BACKEND_ID, format!("create workdir: {e}")))?;
        self.sandboxes.lock().await.insert(
            id.clone(),
            LiveSandbox {
                workdir,
                policy: policy.clone(),
                child: None,
            },
        );
        Ok(SandboxHandle::provisioned(id))
    }

    async fn start(&self, handle: &SandboxHandle) -> Result<SandboxHandle, BackendError> {
        // Read what we need, then release the lock before the spawn await.
        let (workdir, env) = {
            let map = self.sandboxes.lock().await;
            let sb = map
                .get(&handle.id)
                .ok_or_else(|| BackendError::UnknownSandbox(handle.id.to_string()))?;
            (sb.workdir.clone(), self.env_for(&sb.workdir, &sb.policy))
        };
        let (child, endpoint) = spawn_ready(
            BACKEND_ID,
            &self.launch_command,
            &workdir,
            &env,
            &self.ready_marker,
        )
        .await?;
        // Re-lock only to store the child.
        match self.sandboxes.lock().await.get_mut(&handle.id) {
            Some(sb) => sb.child = Some(child),
            // Stopped concurrently; dropping `child` reaps it via kill_on_drop.
            None => return Err(BackendError::UnknownSandbox(handle.id.to_string())),
        }
        Ok(handle.running(endpoint))
    }

    async fn exec(
        &self,
        handle: &SandboxHandle,
        req: ExecRequest,
    ) -> Result<ExecOutput, BackendError> {
        let (workdir, timeout) = {
            let map = self.sandboxes.lock().await;
            let sb = map
                .get(&handle.id)
                .ok_or_else(|| BackendError::UnknownSandbox(handle.id.to_string()))?;
            (sb.workdir.clone(), sb.policy.limits.exec_timeout_secs)
        };
        let (program, args) = req
            .command
            .split_first()
            .ok_or_else(|| exec_err(BACKEND_ID, "empty exec command".into()))?;
        let mut cmd = tokio::process::Command::new(program);
        // kill_on_drop so a timed-out exec doesn't orphan a runaway child.
        cmd.args(args).current_dir(&workdir).kill_on_drop(true);
        let run = cmd.output();
        let out = if timeout == 0 {
            run.await
        } else {
            tokio::time::timeout(Duration::from_secs(timeout), run)
                .await
                .map_err(|_| BackendError::Timeout {
                    backend: BACKEND_ID,
                    op: "exec",
                    secs: timeout,
                })?
        }
        .map_err(|e| exec_err(BACKEND_ID, format!("run `{program}`: {e}")))?;
        Ok(ExecOutput {
            exit_code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    async fn snapshot(&self, handle: &SandboxHandle) -> Result<SnapshotId, BackendError> {
        let (workdir, policy) = {
            let map = self.sandboxes.lock().await;
            let sb = map
                .get(&handle.id)
                .ok_or_else(|| BackendError::UnknownSandbox(handle.id.to_string()))?;
            (sb.workdir.clone(), sb.policy.clone())
        };
        let snap = SnapshotId(format!("snap-{}", SandboxId::new().0));
        let dir = self.snap_dir(&snap);
        copy_tree(BACKEND_ID, snapshot_err, workdir, dir.clone()).await?;
        let policy_json = serde_json::to_vec(&policy)
            .map_err(|e| snapshot_err(BACKEND_ID, format!("serialize policy: {e}")))?;
        tokio::fs::write(dir.join(".sandbox-policy.json"), policy_json)
            .await
            .map_err(|e| snapshot_err(BACKEND_ID, format!("write policy: {e}")))?;
        Ok(snap)
    }

    async fn restore(&self, snapshot: &SnapshotId) -> Result<SandboxHandle, BackendError> {
        let snap = self.snap_dir(snapshot);
        if !snap.is_dir() {
            return Err(restore_err(BACKEND_ID, format!("no snapshot `{snapshot}`")));
        }
        // Propagate a corrupt/unreadable policy — never silently restore under
        // a different (more permissive) posture than was snapshotted.
        let bytes = tokio::fs::read(snap.join(".sandbox-policy.json"))
            .await
            .map_err(|e| restore_err(BACKEND_ID, format!("read snapshot policy: {e}")))?;
        let policy: SandboxPolicy = serde_json::from_slice(&bytes)
            .map_err(|e| restore_err(BACKEND_ID, format!("parse snapshot policy: {e}")))?;
        let id = SandboxId::new();
        let workdir = self.sb_dir(&id);
        copy_tree(BACKEND_ID, restore_err, snap, workdir.clone()).await?;
        self.sandboxes.lock().await.insert(
            id.clone(),
            LiveSandbox {
                workdir,
                policy,
                child: None,
            },
        );
        self.start(&SandboxHandle::provisioned(id)).await
    }

    async fn stop(&self, handle: &SandboxHandle) -> Result<(), BackendError> {
        let sb = self.sandboxes.lock().await.remove(&handle.id);
        let Some(mut sb) = sb else {
            return Err(BackendError::UnknownSandbox(handle.id.to_string()));
        };
        if let Some(mut child) = sb.child.take() {
            child
                .kill()
                .await
                .map_err(|e| BackendError::Stop {
                    backend: BACKEND_ID,
                    detail: format!("kill child: {e}"),
                })?;
        }
        tokio::fs::remove_dir_all(&sb.workdir)
            .await
            .map_err(|e| BackendError::Stop {
                backend: BACKEND_ID,
                detail: format!("remove workdir: {e}"),
            })?;
        Ok(())
    }
}

fn provision_err(backend: &'static str, detail: String) -> BackendError {
    BackendError::Provision { backend, detail }
}
fn exec_err(backend: &'static str, detail: String) -> BackendError {
    BackendError::Exec { backend, detail }
}
fn snapshot_err(backend: &'static str, detail: String) -> BackendError {
    BackendError::Snapshot { backend, detail }
}
fn restore_err(backend: &'static str, detail: String) -> BackendError {
    BackendError::Restore { backend, detail }
}
