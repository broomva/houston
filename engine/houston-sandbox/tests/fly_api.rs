//! `fly` backend tests. The Fly Machines API is faked with `wiremock`, so we
//! verify request routing, request body shape, and response parsing for the
//! whole lifecycle without a real Fly account. (Live Fly correctness is the
//! Chunk-2 gate.)

use houston_sandbox::{
    backend, BackendConfig, BackendError, EgressMode, ExecRequest, NetPolicy, SandboxPolicy,
};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cfg(base: String) -> BackendConfig {
    BackendConfig {
        fly_token: Some("test-token".into()),
        fly_app: Some("app".into()),
        fly_image: Some("registry.fly.io/houston:latest".into()),
        fly_api_base: Some(base),
        ..Default::default()
    }
}

/// A policy the fly backend can fully enforce in Chunk 1 (egress = Allow, no
/// extra mounts). Default `exec_timeout_secs` stays 300.
fn allow_policy() -> SandboxPolicy {
    SandboxPolicy {
        net: NetPolicy {
            egress: EgressMode::Allow,
            allow_hosts: Vec::new(),
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn missing_token_is_not_configured_not_a_silent_fallback() {
    // Ensure no ambient creds leak in from the dev/CI environment.
    std::env::remove_var("FLY_API_TOKEN");
    let result = backend("fly").unwrap().connect(&BackendConfig::default());
    assert!(
        matches!(result, Err(BackendError::NotConfigured { backend: "fly", .. })),
        "expected NotConfigured for missing creds"
    );
}

#[tokio::test]
async fn provision_rejects_unenforceable_egress_rather_than_weakening_it() {
    // A real Fly call must never happen for a policy we can't honor, so no
    // mock server is needed — the guard fires before any HTTP.
    let runner = backend("fly")
        .unwrap()
        .connect(&cfg("http://127.0.0.1:1".into()))
        .unwrap();
    // Default policy has egress = AllowList, which fly can't enforce yet.
    let err = match runner.provision(&SandboxPolicy::default()).await {
        Err(e) => e,
        Ok(_) => panic!("default (restrictive) policy must be rejected, not silently widened"),
    };
    match err {
        BackendError::Provision { detail, .. } => assert!(detail.contains("egress"), "{detail}"),
        other => panic!("expected Provision egress error, got {other:?}"),
    }
}

#[tokio::test]
async fn drives_full_lifecycle_against_mock_fly_api() {
    let server = MockServer::start().await;
    let base = "/apps/app/machines";

    Mock::given(method("POST"))
        .and(path(base))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"id": "m-123", "state": "created"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{base}/m-123/start")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&server)
        .await;
    // Lock the exec wire shape: `cmd` STRING (not a `command` array) + timeout.
    Mock::given(method("POST"))
        .and(path(format!("{base}/m-123/exec")))
        .and(body_json(serde_json::json!({"cmd": "echo hi", "timeout": 300})))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"exit_code": 0, "stdout": "hi\n", "stderr": ""}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{base}/m-123/suspend")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("{base}/m-123/stop")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{base}/m-123")))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let runner = backend("fly").unwrap().connect(&cfg(server.uri())).unwrap();

    let handle = runner.provision(&allow_policy()).await.unwrap();
    assert_eq!(handle.id.0, "m-123");

    let handle = runner.start(&handle).await.unwrap();
    assert!(handle.endpoint.is_some());

    let out = runner
        .exec(&handle, ExecRequest::new(["echo", "hi"]))
        .await
        .unwrap();
    assert!(out.success());
    assert_eq!(out.stdout, "hi\n");

    let snap = runner.snapshot(&handle).await.unwrap();
    assert_eq!(snap.0, "m-123"); // suspend → restore by machine id

    let restored = runner.restore(&snap).await.unwrap();
    assert_eq!(restored.id.0, "m-123");

    runner.stop(&handle).await.unwrap();
}

#[tokio::test]
async fn non_2xx_surfaces_a_typed_error_with_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/apps/app/machines"))
        .respond_with(ResponseTemplate::new(422).set_body_string("image not found"))
        .mount(&server)
        .await;

    let runner = backend("fly").unwrap().connect(&cfg(server.uri())).unwrap();
    let err = match runner.provision(&allow_policy()).await {
        Err(e) => e,
        Ok(_) => panic!("expected provision to fail on 422"),
    };
    match err {
        BackendError::Provision { detail, .. } => {
            assert!(detail.contains("422"), "status surfaced: {detail}");
            assert!(detail.contains("image not found"), "body surfaced: {detail}");
        }
        other => panic!("expected Provision error, got {other:?}"),
    }
}
