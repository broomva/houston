//! `fly` backend — Fly Machines (managed Firecracker), the lean v1 cloud
//! host. One microVM per agent: hard isolation + millisecond
//! snapshot/restore without standing up a Kubernetes cluster.
//!
//! Lifecycle → Fly Machines API mapping:
//! - `provision` → create machine with `skip_launch` (cold-boot cost lands
//!   on `start`, not here)
//! - `start`     → start machine, derive the engine endpoint
//! - `exec`      → machine exec endpoint
//! - `snapshot`  → **suspend** (memory snapshot to disk — the scale-to-zero
//!   primitive); the [`SnapshotId`] is the suspended machine's id
//! - `restore`   → start the suspended machine (resume)
//! - `stop`      → stop + destroy (terminal teardown)
//!
//! Credentials come from [`BackendConfig`] or the `FLY_API_TOKEN` / `FLY_APP`
//! / `FLY_IMAGE` env vars. With no token, [`SandboxBackend::connect`]
//! returns [`BackendError::NotConfigured`] — never a silent fallback.
//!
//! Policy enforcement boundary (Chunk 1): this backend enforces the
//! `limits` (guest sizing) and `identity.env` of a [`crate::SandboxPolicy`].
//! It does **not** yet wire Fly's egress controls or extra volume mounts, so
//! `provision` rejects a policy that asks for either rather than silently
//! producing a more-open machine than declared (see `lifecycle`).

mod api;
mod lifecycle;

use crate::error::BackendError;
use crate::registry::BackendConfig;
use crate::runner::{SandboxBackend, SandboxRunner};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) const BACKEND_ID: &str = "fly";
const DEFAULT_API_BASE: &str = "https://api.machines.dev/v1";

/// The registered `fly` backend singleton.
pub struct FlyBackend;

/// Singleton instance held in the registry.
pub static FLY: FlyBackend = FlyBackend;

impl SandboxBackend for FlyBackend {
    fn id(&self) -> &'static str {
        BACKEND_ID
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["fly.io", "firecracker"]
    }
    fn description(&self) -> &'static str {
        "Fly Machines (managed Firecracker) — one microVM per agent"
    }
    fn connect(&self, config: &BackendConfig) -> Result<Arc<dyn SandboxRunner>, BackendError> {
        let token = config
            .fly_token
            .clone()
            .or_else(|| std::env::var("FLY_API_TOKEN").ok())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| BackendError::NotConfigured {
                backend: BACKEND_ID,
                detail: "set BackendConfig.fly_token or $FLY_API_TOKEN".into(),
            })?;
        let app = config
            .fly_app
            .clone()
            .or_else(|| std::env::var("FLY_APP").ok())
            .filter(|a| !a.is_empty())
            .ok_or_else(|| BackendError::NotConfigured {
                backend: BACKEND_ID,
                detail: "set BackendConfig.fly_app or $FLY_APP".into(),
            })?;
        let image = config
            .fly_image
            .clone()
            .or_else(|| std::env::var("FLY_IMAGE").ok())
            .filter(|i| !i.is_empty())
            .ok_or_else(|| BackendError::NotConfigured {
                backend: BACKEND_ID,
                detail: "set BackendConfig.fly_image or $FLY_IMAGE".into(),
            })?;
        let base = config
            .fly_api_base
            .clone()
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

        Ok(Arc::new(FlyRunner {
            http: reqwest::Client::new(),
            base,
            app,
            image,
            token,
            exec_timeouts: Mutex::new(HashMap::new()),
        }))
    }
}

/// Live controller talking to the Fly Machines API. Lifecycle impl lives in
/// [`lifecycle`].
pub struct FlyRunner {
    http: reqwest::Client,
    base: String,
    app: String,
    image: String,
    token: String,
    /// Per-machine exec wall-clock ceiling (seconds), from the policy at
    /// provision time. `0` / absent means no client-side timeout. Mirrors how
    /// the `local` backend applies `limits.exec_timeout_secs`.
    exec_timeouts: Mutex<HashMap<String, u64>>,
}

impl FlyRunner {
    fn machines_url(&self) -> String {
        format!("{}/apps/{}/machines", self.base, self.app)
    }

    /// POST `path` (relative to the machines collection) with an optional
    /// JSON body, mapping transport + non-2xx into a typed `op` error.
    async fn post(
        &self,
        op_err: fn(String) -> BackendError,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response, BackendError> {
        let url = if path.is_empty() {
            self.machines_url()
        } else {
            format!("{}/{}", self.machines_url(), path)
        };
        let mut req = self.http.post(&url).bearer_auth(&self.token);
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| op_err(format!("POST {url}: {e}")))?;
        Self::ensure_ok(op_err, resp).await
    }

    /// DELETE a machine, mapping failures into a typed `op` error.
    async fn delete(
        &self,
        op_err: fn(String) -> BackendError,
        id: &str,
    ) -> Result<(), BackendError> {
        let url = format!("{}/{}", self.machines_url(), id);
        let resp = self
            .http
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| op_err(format!("DELETE {url}: {e}")))?;
        Self::ensure_ok(op_err, resp).await.map(|_| ())
    }

    /// Turn a non-2xx response into a typed error carrying the body.
    async fn ensure_ok(
        op_err: fn(String) -> BackendError,
        resp: reqwest::Response,
    ) -> Result<reqwest::Response, BackendError> {
        if resp.status().is_success() {
            return Ok(resp);
        }
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("<error body unreadable: {e}>"));
        Err(op_err(format!("Fly API {status}: {body}")))
    }

    /// The agent engine's address on the Fly private network.
    fn endpoint(&self) -> String {
        format!("http://{}.internal:8080", self.app)
    }
}
