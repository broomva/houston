//! End-to-end lifecycle test for the `local` backend against real host
//! processes. Unix-gated: the compute plane is Linux/Firecracker and the
//! test relies on `sleep`/`echo` being on PATH.
#![cfg(unix)]

use houston_sandbox::{
    backend, BackendConfig, ExecRequest, SandboxHandle, SandboxId, SandboxState,
};

/// A config that spawns a portable long-running process and treats "spawned"
/// as "ready" (no engine banner needed in tests).
fn spawn_only_cfg() -> BackendConfig {
    BackendConfig {
        launch_command: Some(vec!["sleep".into(), "30".into()]),
        ready_marker: Some(String::new()),
        ..Default::default()
    }
}

#[tokio::test]
async fn full_lifecycle_provision_to_stop() {
    let runner = backend("local").unwrap().connect(&spawn_only_cfg()).unwrap();

    let handle = runner
        .provision(&houston_sandbox::SandboxPolicy::default())
        .await
        .unwrap();
    assert_eq!(handle.state, SandboxState::Provisioned);

    let handle = runner.start(&handle).await.unwrap();
    assert_eq!(handle.state, SandboxState::Running);

    let out = runner
        .exec(&handle, ExecRequest::new(["echo", "lifecycle-ok"]))
        .await
        .unwrap();
    assert!(out.success(), "echo should exit 0: {out:?}");
    assert!(out.stdout.contains("lifecycle-ok"));

    let snap = runner.snapshot(&handle).await.unwrap();
    let restored = runner.restore(&snap).await.unwrap();
    assert_eq!(restored.state, SandboxState::Running);
    assert_ne!(restored.id, handle.id, "restore makes a new sandbox");

    runner.stop(&handle).await.unwrap();
    runner.stop(&restored).await.unwrap();
}

#[tokio::test]
async fn exec_carries_nonzero_exit() {
    let runner = backend("local").unwrap().connect(&spawn_only_cfg()).unwrap();
    let handle = runner
        .provision(&houston_sandbox::SandboxPolicy::default())
        .await
        .unwrap();
    let handle = runner.start(&handle).await.unwrap();
    let out = runner
        .exec(&handle, ExecRequest::new(["false"]))
        .await
        .unwrap();
    assert!(!out.success());
    runner.stop(&handle).await.unwrap();
}

#[tokio::test]
async fn unknown_sandbox_is_rejected() {
    let runner = backend("local").unwrap().connect(&spawn_only_cfg()).unwrap();
    let ghost = SandboxHandle::provisioned(SandboxId("does-not-exist".into()));
    assert!(runner.exec(&ghost, ExecRequest::new(["true"])).await.is_err());
    assert!(runner.stop(&ghost).await.is_err());
}
