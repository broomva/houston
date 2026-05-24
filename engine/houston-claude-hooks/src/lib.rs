//! Install / uninstall Houston-managed hooks in Claude Code's
//! `settings.json` (Phase 7 of RFC #248, `advanced.claude_hooks`).
//!
//! ## What this crate does
//!
//! Claude Code reads `~/.claude/settings.json` and runs a configured
//! shell command at each tool-use lifecycle point (`PreToolUse`,
//! `PostToolUse`, `Stop`, `Notification`, ...). This crate writes a
//! small set of "Houston hooks" into that file: each one appends a
//! JSON line to `~/.houston/claude-hooks/events.jsonl` so the user
//! can `tail -f` it and watch every tool call Claude Code makes,
//! independently of Houston's own session pipeline.
//!
//! The flag (`advanced.claude_hooks`) is enforced engine-side because
//! the install path is a filesystem write to a file shared with
//! Claude Code itself. A UI-only gate would not stop a malicious
//! caller from hitting the route directly.
//!
//! ## Why the hook tag
//!
//! Claude's hook schema does not allow arbitrary keys in a hook
//! entry, so we tag Houston-installed hooks by appending the literal
//! `# houston-hook` comment to the command string. Uninstall greps
//! for that comment and removes only those entries, leaving anything
//! the user added by hand intact.
//!
//! ## Module split (each stays under the 200-line cap)
//!
//! - `lib.rs` — public surface, `HookStatus`, error type, re-exports.
//! - `settings` — read / merge / write `settings.json` atomically.
//! - `commands` — the literal command strings + matcher set.

mod commands;
mod settings;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use commands::{events_log_path, HOOK_TAG};
pub use settings::{install, settings_path_for_home, uninstall, ParseError};

/// Hook install status, as reported by `read_status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookStatus {
    /// Resolved path to `settings.json` — always returned, even when the
    /// file does not exist yet, so the UI can show "where this would
    /// land if installed".
    pub settings_path: PathBuf,
    /// `true` when `settings.json` exists on disk.
    pub settings_exists: bool,
    /// Number of Houston-tagged hook entries present in the file.
    /// `0` ⇒ not installed. Non-zero ⇒ installed (and equal to the
    /// length of `commands::HOOK_EVENTS` after a clean install).
    pub houston_hook_count: usize,
    /// Total hook entries the file contains across all events,
    /// Houston-tagged or not. Lets the UI distinguish "Houston is
    /// not installed in an otherwise hook-using file" from "the
    /// file has no hooks at all".
    pub total_hook_count: usize,
    /// Resolved path to the JSONL events file that the Houston hooks
    /// append to. Returned even when the hooks are not installed so
    /// the UI can show where the log would land.
    pub events_log_path: PathBuf,
}

/// Errors that escape the public API. Internal IO errors are wrapped
/// here so callers don't need to depend on `std::io` directly.
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("IO error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("settings.json at {path} is not valid JSON: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ParseError,
    },
}

/// Read the current install status. Never fails on a missing file —
/// returns `settings_exists: false` with zero counts. Only IO and
/// parse errors propagate.
pub fn read_status(home: &Path) -> Result<HookStatus, HookError> {
    let settings_path = settings_path_for_home(home);
    let events_log_path = events_log_path(home);
    if !settings_path.exists() {
        return Ok(HookStatus {
            settings_path,
            settings_exists: false,
            houston_hook_count: 0,
            total_hook_count: 0,
            events_log_path,
        });
    }
    let bytes = std::fs::read(&settings_path).map_err(|e| HookError::Io {
        path: settings_path.clone(),
        source: e,
    })?;
    let counts = settings::count_hooks(&bytes).map_err(|e| HookError::Parse {
        path: settings_path.clone(),
        source: e,
    })?;
    Ok(HookStatus {
        settings_path,
        settings_exists: true,
        houston_hook_count: counts.houston,
        total_hook_count: counts.total,
        events_log_path,
    })
}
