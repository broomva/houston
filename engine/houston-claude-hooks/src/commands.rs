//! The literal hook command strings + matcher set that Houston writes
//! into Claude Code's `settings.json`.
//!
//! Keeping these in one file makes the install/uninstall logic easy
//! to audit ("what exactly does Houston put on disk?") and gives the
//! tag a single source of truth.

use std::path::{Path, PathBuf};

/// Comment we append to every Houston-installed hook command so
/// uninstall can find and remove exactly our entries. Stays in the
/// command string itself because Claude Code's hook schema doesn't
/// allow arbitrary keys on the hook entry.
pub const HOOK_TAG: &str = "# houston-hook";

/// The events Houston subscribes to. Order is the order they are
/// written into `settings.json`. Each one gets one hook entry with
/// matcher `*` and a shell command that appends the lifecycle payload
/// to `events_log_path(home)` as a single JSON line.
pub const HOOK_EVENTS: &[&str] = &["PreToolUse", "PostToolUse", "Stop", "Notification"];

/// The matcher Houston installs for each event. `*` = match every
/// tool. We intentionally do not let the user narrow the matcher in
/// v1: a single "see everything" install is the smallest valuable
/// thing we can ship. Narrowing comes in a future iteration.
pub const HOOK_MATCHER: &str = "*";

/// JSONL append target. `cat -` reads the JSON payload Claude Code
/// pipes to the hook on stdin, and the `>>` keeps the file growing.
/// `mkdir -p` is idempotent — first install creates the directory;
/// later runs do nothing. The trailing tag is what uninstall keys on.
pub fn hook_command(events_log: &Path) -> String {
    // Use `printf` instead of `echo` so any literal `\` or `-n` in
    // the payload is preserved verbatim. `cat -` already trails a
    // newline; we don't append another.
    let log = events_log.to_string_lossy();
    format!("mkdir -p \"$(dirname '{log}')\" && cat - >> '{log}' {HOOK_TAG}")
}

/// The JSONL file Houston's hooks append to. Lives under
/// `~/.houston/claude-hooks/events.jsonl` so it sits next to the rest
/// of Houston's per-user state and is easy to find / tail / rotate.
pub fn events_log_path(home: &Path) -> PathBuf {
    home.join(".houston")
        .join("claude-hooks")
        .join("events.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn hook_command_contains_log_path_and_tag() {
        let cmd = hook_command(&PathBuf::from("/tmp/x/events.jsonl"));
        assert!(cmd.contains("/tmp/x/events.jsonl"), "{cmd}");
        assert!(cmd.ends_with(HOOK_TAG), "{cmd}");
        assert!(cmd.contains("mkdir -p"), "{cmd}");
    }

    #[test]
    fn events_log_path_under_houston_dir() {
        let log = events_log_path(&PathBuf::from("/home/u"));
        assert_eq!(
            log,
            PathBuf::from("/home/u/.houston/claude-hooks/events.jsonl")
        );
    }

    #[test]
    fn hook_events_set_is_stable() {
        // Order is part of the contract — `settings.json` diffs cleanly
        // across installs only if we write the same keys in the same order.
        assert_eq!(
            HOOK_EVENTS,
            &["PreToolUse", "PostToolUse", "Stop", "Notification"]
        );
    }
}
