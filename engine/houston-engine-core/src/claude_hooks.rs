//! Engine-core facade over `houston-claude-hooks`. Phase 7 of RFC #248
//! (`advanced.claude_hooks`).
//!
//! Routes always-on; the UI gate is a frontend feature flag, and the
//! preference check happens engine-side too — see
//! `routes/claude_hooks.rs` which reads `advanced.claude_hooks` from
//! the preferences DB before performing the side-effecting install /
//! uninstall.

use crate::error::{CoreError, CoreResult};
use houston_claude_hooks::{self as hooks, HookError, HookStatus};
use std::path::Path;

pub use houston_claude_hooks::HookStatus as ClaudeHookStatus;

/// Read install status. Never fails on a missing settings.json — it
/// reports `settings_exists: false` with zero counts so the UI can
/// show "not installed" without an extra round-trip.
pub fn status(home: &Path) -> CoreResult<HookStatus> {
    hooks::read_status(home).map_err(map_hook_error)
}

/// Install Houston's hooks into `~/.claude/settings.json`. Idempotent:
/// re-running after a successful install rewrites the same bytes.
pub fn install(home: &Path) -> CoreResult<HookStatus> {
    hooks::install(home).map_err(|e| CoreError::Internal(format!("claude_hooks install: {e}")))?;
    status(home)
}

/// Remove every Houston-tagged hook entry. Leaves user-installed
/// entries untouched. Returns the post-uninstall status so the UI can
/// re-render without a follow-up GET.
pub fn uninstall(home: &Path) -> CoreResult<HookStatus> {
    hooks::uninstall(home)
        .map_err(|e| CoreError::Internal(format!("claude_hooks uninstall: {e}")))?;
    status(home)
}

fn map_hook_error(e: HookError) -> CoreError {
    match e {
        HookError::Io { path, source } => {
            CoreError::Internal(format!("claude_hooks read {}: {source}", path.display()))
        }
        HookError::Parse { path, source } => {
            CoreError::Internal(format!("claude_hooks parse {}: {source}", path.display()))
        }
    }
}
