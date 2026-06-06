//! Backend registry — the same shape as `houston-terminal-manager`'s
//! provider registry: a `const` array of `&'static dyn` singletons, a
//! `Copy` newtype handle, and id/alias lookup. Adding a backend is one
//! import + one array entry.

use crate::backends::{fly, local};
use crate::error::BackendError;
use crate::runner::{SandboxBackend, SandboxRunner};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

/// Every registered isolation backend. Add one by importing its singleton
/// and pushing it here — no other call site changes.
const BACKENDS: &[&dyn SandboxBackend] = &[&local::LOCAL, &fly::FLY];

/// Default backend when nothing is configured. `local` keeps the
/// walking-skeleton spike runnable on a dev box with no cloud account.
const DEFAULT_BACKEND: &dyn SandboxBackend = &local::LOCAL;

/// Runtime configuration passed to [`SandboxBackend::connect`]. Every field
/// is optional; backends ignore what they do not use.
#[derive(Debug, Clone, Default)]
pub struct BackendConfig {
    /// `local`: argv launched as the long-running in-sandbox process.
    /// Defaults to `["houston-engine"]` (resolved on PATH).
    pub launch_command: Option<Vec<String>>,
    /// `local`: stdout substring signalling readiness. When it carries a
    /// `port=<n>` token the runner derives the serving endpoint from it.
    /// Defaults to the engine's `HOUSTON_ENGINE_LISTENING` banner. Set to
    /// `Some(String::new())` to treat "spawned" as "ready" (tests, daemons
    /// that print no banner).
    pub ready_marker: Option<String>,
    /// `fly`: Machines API token. Falls back to `$FLY_API_TOKEN`.
    pub fly_token: Option<String>,
    /// `fly`: target app name. Falls back to `$FLY_APP`.
    pub fly_app: Option<String>,
    /// `fly`: API base URL override (for mocks/tests). Defaults to
    /// `https://api.machines.dev/v1`.
    pub fly_api_base: Option<String>,
    /// `fly`: machine image to boot. Falls back to `$FLY_IMAGE`.
    pub fly_image: Option<String>,
}

/// Look up a backend singleton by id or alias. `None` for unknown ids.
fn get(id: &str) -> Option<&'static dyn SandboxBackend> {
    let lower = id.to_lowercase();
    BACKENDS
        .iter()
        .copied()
        .find(|b| b.id() == lower || b.aliases().iter().any(|a| *a == lower))
}

/// Resolve a backend by id or alias.
pub fn backend(id: &str) -> Result<Backend, BackendError> {
    get(id)
        .map(Backend)
        .ok_or_else(|| BackendError::UnknownBackend(id.to_string()))
}

/// All registered backends, in registration order.
pub fn all_backends() -> Vec<Backend> {
    BACKENDS.iter().copied().map(Backend).collect()
}

/// The default backend handle.
pub fn default_backend() -> Backend {
    Backend(DEFAULT_BACKEND)
}

/// Identifier-like handle to a registered [`SandboxBackend`]. `Copy` (a fat
/// pointer) and serializes as its [`SandboxBackend::id`] string.
#[derive(Clone, Copy)]
pub struct Backend(&'static dyn SandboxBackend);

impl Backend {
    /// Backend id (e.g. `"fly"`).
    pub fn id(self) -> &'static str {
        self.0.id()
    }

    /// One-line description.
    pub fn description(self) -> &'static str {
        self.0.description()
    }

    /// Build a live runner from config. See [`SandboxBackend::connect`].
    pub fn connect(self, config: &BackendConfig) -> Result<Arc<dyn SandboxRunner>, BackendError> {
        self.0.connect(config)
    }
}

impl fmt::Debug for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Backend").field(&self.0.id()).finish()
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.id())
    }
}

impl PartialEq for Backend {
    fn eq(&self, other: &Self) -> bool {
        self.0.id() == other.0.id()
    }
}

impl Eq for Backend {}

impl FromStr for Backend {
    type Err = BackendError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        backend(s)
    }
}

impl Serialize for Backend {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.0.id())
    }
}

impl<'de> Deserialize<'de> for Backend {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_backends() {
        assert_eq!(backend("local").unwrap().id(), "local");
        assert_eq!(backend("fly").unwrap().id(), "fly");
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(backend("LOCAL").unwrap().id(), "local");
    }

    #[test]
    fn unknown_backend_errors() {
        let e = backend("nope").unwrap_err();
        assert!(matches!(e, BackendError::UnknownBackend(_)));
    }

    #[test]
    fn default_is_local() {
        assert_eq!(default_backend().id(), "local");
    }

    #[test]
    fn registry_lists_all() {
        let ids: Vec<_> = all_backends().iter().map(|b| b.id()).collect();
        assert!(ids.contains(&"local"));
        assert!(ids.contains(&"fly"));
    }

    #[test]
    fn serde_round_trip_via_id() {
        let b = backend("fly").unwrap();
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "\"fly\"");
        let back: Backend = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id(), "fly");
    }
}
