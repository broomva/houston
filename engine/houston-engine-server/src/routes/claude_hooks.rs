//! `/v1/claude-hooks/*` — Houston-managed Claude Code hook lifecycle.
//! Phase 7 of RFC #248 (`advanced.claude_hooks`).
//!
//! The flag is enforced engine-side because install / uninstall write
//! to `~/.claude/settings.json`, a file shared with the user's Claude
//! Code CLI. A UI-only gate would not stop a malicious caller from
//! POSTing directly. Status is allowed unconditionally — it only
//! reads the file.

use crate::routes::error::ApiError;
use crate::state::ServerState;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use houston_engine_core::claude_hooks::{self, ClaudeHookStatus};
use houston_engine_core::{preferences, CoreError};
use houston_engine_protocol::ErrorCode;
use std::sync::Arc;

/// Preference key the UI toggles. Must match `FLAG_REGISTRY` in
/// `app/src/lib/featureFlags.ts`. String "true" / "false" / unset.
const FLAG_KEY: &str = "advanced.claude_hooks";

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/claude-hooks/status", get(status))
        .route("/claude-hooks/install", post(install))
        .route("/claude-hooks/uninstall", post(uninstall))
}

async fn status(State(st): State<Arc<ServerState>>) -> Result<Json<ClaudeHookStatus>, ApiError> {
    Ok(Json(claude_hooks::status(st.engine.paths.home())?))
}

async fn install(State(st): State<Arc<ServerState>>) -> Result<Json<ClaudeHookStatus>, ApiError> {
    ensure_flag_on(&st).await?;
    Ok(Json(claude_hooks::install(st.engine.paths.home())?))
}

async fn uninstall(State(st): State<Arc<ServerState>>) -> Result<Json<ClaudeHookStatus>, ApiError> {
    // Uninstall stays allowed even when the flag is off — a user who
    // toggled the flag off after install must still be able to clean
    // up. Otherwise we leak hook entries the user wants gone.
    Ok(Json(claude_hooks::uninstall(st.engine.paths.home())?))
}

async fn ensure_flag_on(st: &ServerState) -> Result<(), CoreError> {
    let value = preferences::get(&st.engine.db, FLAG_KEY).await?;
    match value.as_deref() {
        Some("true") => Ok(()),
        _ => Err(CoreError::Labeled {
            code: ErrorCode::Forbidden,
            kind: "claude_hooks_disabled",
            message: "advanced.claude_hooks is off — toggle it on in Settings > Advanced first"
                .to_string(),
        }),
    }
}
