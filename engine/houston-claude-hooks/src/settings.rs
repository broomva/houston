//! Read, merge, write Claude Code's `settings.json` for
//! `houston-claude-hooks`.
//!
//! Invariants this module enforces:
//!
//! 1. **Preserve every key we don't touch.** The user's other
//!    settings (model defaults, theme, custom MCP servers,
//!    user-installed hooks, ...) round-trip unchanged.
//! 2. **Tagged-entry uninstall.** We only remove hook entries whose
//!    `command` contains `HOOK_TAG`. Anything the user wrote stays.
//! 3. **Atomic write.** Write to a sibling temp file then `rename`
//!    so a crash mid-write never leaves a half-written settings.json.
//! 4. **No empty containers left behind.** After uninstall, if the
//!    file ends up with `"hooks": {}` or a key with `[]`, we strip
//!    those so the diff is identical to "Houston was never here".

use crate::commands::{events_log_path, hook_command, HOOK_EVENTS, HOOK_MATCHER, HOOK_TAG};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// Resolve the `settings.json` path Claude Code reads on this host.
/// We standardize on `~/.claude/settings.json` for every platform —
/// that is the location Claude Code itself documents.
pub fn settings_path_for_home(home: &Path) -> PathBuf {
    home.join(".claude").join("settings.json")
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("not valid JSON: {0}")]
    NotJson(#[from] serde_json::Error),
    #[error("top-level value is not a JSON object")]
    NotObject,
}

#[derive(Debug)]
pub struct HookCounts {
    pub houston: usize,
    pub total: usize,
}

/// Count Houston-tagged and total hook entries in `settings.json` bytes.
/// A missing or empty `hooks` field returns zeros. Used by
/// `read_status` so the UI can show install state without writing.
pub fn count_hooks(bytes: &[u8]) -> Result<HookCounts, ParseError> {
    let value: Value = serde_json::from_slice(bytes)?;
    let obj = value.as_object().ok_or(ParseError::NotObject)?;
    let Some(hooks) = obj.get("hooks").and_then(Value::as_object) else {
        return Ok(HookCounts {
            houston: 0,
            total: 0,
        });
    };
    let mut houston = 0;
    let mut total = 0;
    for entries in hooks.values().filter_map(Value::as_array) {
        for entry in entries {
            let Some(inner) = entry.get("hooks").and_then(Value::as_array) else {
                continue;
            };
            for hook in inner {
                total += 1;
                if hook
                    .get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains(HOOK_TAG))
                {
                    houston += 1;
                }
            }
        }
    }
    Ok(HookCounts { houston, total })
}

/// Install Houston's hook set into the user's `settings.json`. Creates
/// the parent directory and the file if missing; merges into the
/// existing object otherwise. Idempotent: re-running after an install
/// produces the exact same on-disk bytes (we de-dupe Houston entries
/// before re-writing).
pub fn install(home: &Path) -> std::io::Result<()> {
    let settings_path = settings_path_for_home(home);
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let events_log = events_log_path(home);
    let mut root = load_object(&settings_path)?;
    let hooks = root
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "settings.json `hooks` is not a JSON object",
            )
        })?;
    for event in HOOK_EVENTS {
        let entries = hooks
            .entry((*event).to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("settings.json `hooks.{event}` is not a JSON array"),
                )
            })?;
        // Drop any pre-existing Houston entries on this event so re-running
        // install is a no-op rather than an accidental duplicate.
        entries.retain(|e| !is_houston_entry(e));
        entries.push(houston_entry(&events_log));
    }
    write_atomic(&settings_path, &Value::Object(root))
}

/// Remove every Houston-tagged hook entry. Leaves user-installed
/// entries untouched. Strips emptied containers so the file's diff
/// against pre-install is clean.
pub fn uninstall(home: &Path) -> std::io::Result<()> {
    let settings_path = settings_path_for_home(home);
    if !settings_path.exists() {
        return Ok(());
    }
    let mut root = load_object(&settings_path)?;
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    let keys: Vec<String> = hooks.keys().cloned().collect();
    for key in keys {
        let Some(entries) = hooks.get_mut(&key).and_then(Value::as_array_mut) else {
            continue;
        };
        // Strip the inner hooks array of Houston entries; drop any
        // entry left with an empty inner array.
        entries.retain_mut(|entry| {
            let Some(inner) = entry.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            inner.retain(|h| {
                !h.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains(HOOK_TAG))
            });
            !inner.is_empty()
        });
        if entries.is_empty() {
            hooks.remove(&key);
        }
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    write_atomic(&settings_path, &Value::Object(root))
}

fn load_object(path: &Path) -> std::io::Result<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let bytes = std::fs::read(path)?;
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(Map::new());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("settings.json is not valid JSON: {e}"),
        )
    })?;
    match value {
        Value::Object(m) => Ok(m),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "settings.json top-level value must be an object",
        )),
    }
}

fn is_houston_entry(entry: &Value) -> bool {
    let Some(inner) = entry.get("hooks").and_then(Value::as_array) else {
        return false;
    };
    inner.iter().all(|h| {
        h.get("command")
            .and_then(Value::as_str)
            .is_some_and(|c| c.contains(HOOK_TAG))
    })
}

fn houston_entry(events_log: &Path) -> Value {
    json!({
        "matcher": HOOK_MATCHER,
        "hooks": [
            {
                "type": "command",
                "command": hook_command(events_log),
            }
        ],
    })
}

fn write_atomic(path: &Path, value: &Value) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("serialize settings.json: {e}"),
        )
    })?;
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "settings.json has no parent",
        )
    })?;
    let mut tmp = path.to_path_buf();
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| "settings.json".into());
    name.push(".houston.tmp");
    tmp.set_file_name(name);
    std::fs::create_dir_all(parent)?;
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn read(path: &Path) -> Value {
        let bytes = std::fs::read(path).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn install_creates_file_with_all_events() {
        let dir = tempdir().unwrap();
        install(dir.path()).unwrap();
        let v = read(&settings_path_for_home(dir.path()));
        let hooks = v["hooks"].as_object().unwrap();
        for ev in HOOK_EVENTS {
            assert!(hooks.contains_key(*ev), "missing {ev}");
            let entries = hooks[*ev].as_array().unwrap();
            assert_eq!(entries.len(), 1, "{ev}");
        }
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempdir().unwrap();
        install(dir.path()).unwrap();
        let first = std::fs::read(settings_path_for_home(dir.path())).unwrap();
        install(dir.path()).unwrap();
        let second = std::fs::read(settings_path_for_home(dir.path())).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn install_preserves_user_keys_and_user_hooks() {
        let dir = tempdir().unwrap();
        let path = settings_path_for_home(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
              "model": "claude-sonnet-4-6",
              "hooks": {
                "PreToolUse": [
                  { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo user" } ] }
                ]
              }
            }"#,
        )
        .unwrap();
        install(dir.path()).unwrap();
        let v = read(&path);
        assert_eq!(v["model"], "claude-sonnet-4-6");
        let pre = v["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2, "user entry + Houston entry");
        assert!(pre.iter().any(|e| e["hooks"][0]["command"] == "echo user"));
    }

    #[test]
    fn uninstall_removes_only_houston_entries() {
        let dir = tempdir().unwrap();
        let path = settings_path_for_home(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
              "model": "claude-sonnet-4-6",
              "hooks": {
                "PreToolUse": [
                  { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo user" } ] }
                ]
              }
            }"#,
        )
        .unwrap();
        install(dir.path()).unwrap();
        uninstall(dir.path()).unwrap();
        let v = read(&path);
        assert_eq!(v["model"], "claude-sonnet-4-6");
        let pre = v["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 1);
        assert_eq!(pre[0]["hooks"][0]["command"], "echo user");
    }

    #[test]
    fn uninstall_strips_empty_containers() {
        let dir = tempdir().unwrap();
        install(dir.path()).unwrap();
        uninstall(dir.path()).unwrap();
        let v = read(&settings_path_for_home(dir.path()));
        assert!(v.get("hooks").is_none(), "hooks key should be gone: {v}");
    }

    #[test]
    fn uninstall_missing_file_is_noop() {
        let dir = tempdir().unwrap();
        uninstall(dir.path()).unwrap();
        assert!(!settings_path_for_home(dir.path()).exists());
    }

    #[test]
    fn count_hooks_distinguishes_houston_and_user() {
        let dir = tempdir().unwrap();
        let path = settings_path_for_home(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
              "hooks": {
                "PreToolUse": [
                  { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo user" } ] }
                ]
              }
            }"#,
        )
        .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let c = count_hooks(&bytes).unwrap();
        assert_eq!(c.houston, 0);
        assert_eq!(c.total, 1);
        install(dir.path()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let c = count_hooks(&bytes).unwrap();
        assert_eq!(c.houston, HOOK_EVENTS.len());
        assert_eq!(c.total, HOOK_EVENTS.len() + 1);
    }

    #[test]
    fn count_hooks_handles_missing_hooks_field() {
        let c = count_hooks(b"{\"model\":\"x\"}").unwrap();
        assert_eq!(c.houston, 0);
        assert_eq!(c.total, 0);
    }

    #[test]
    fn parse_error_on_non_object_root() {
        let err = count_hooks(b"[]").unwrap_err();
        assert!(matches!(err, ParseError::NotObject));
    }
}
