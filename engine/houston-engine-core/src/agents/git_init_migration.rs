//! Boot-time migration: ensure every existing Houston agent folder is a
//! git working tree.
//!
//! The companion create-time fix (`agents_crud::create` → `git::ensure_repo_sync`)
//! covers agents created after the fix lands, but every agent created
//! *before* still hits the `git_not_a_repo` toast the moment a git-aware
//! surface (`git_panel`, status/log/diff routes) touches it. This
//! migration walks the workspaces tree on engine boot and runs
//! `ensure_repo_sync` on each agent folder so the same idempotent
//! contract applies to historical agents.
//!
//! Pattern mirrors [`crate::workspaces::migrate_workspace_provider_into_agents`]:
//! walk workspaces from [`crate::workspaces::io::read_all`], visit each
//! agent directory inside, fix in place. Idempotent — re-running on an
//! already-migrated tree is a constant-time check per agent (just
//! `git rev-parse --git-dir`) and reports every dir as `AlreadyAGitRepo`.
//!
//! Failures (per-agent) are collected into the returned [`MigrationStats`]
//! rather than aborting the whole migration — one broken agent dir
//! shouldn't stop the rest from getting fixed. The boot-time caller
//! logs the stats at info level so the user sees what was done.

use crate::error::CoreResult;
use crate::git::{ensure_repo_sync, EnsureRepoOutcome};
use crate::workspaces;
use std::fs;
use std::path::Path;

/// Tally of what the migration did on this boot — surfaced at info
/// level so the user can see whether a fix landed silently or
/// something genuinely needed attention.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationStats {
    pub workspaces_scanned: usize,
    pub agents_scanned: usize,
    pub initialized: usize,
    pub already_a_repo: usize,
    /// Per-agent errors. The migration logs each individually and
    /// continues; the user sees the count + reasons in the boot log.
    pub errors: Vec<String>,
}

/// Walk every workspace in `root` and call [`ensure_repo_sync`] on each
/// agent folder. Returns aggregate stats — errors are accumulated, not
/// thrown, so a single broken agent dir doesn't block the rest.
///
/// An "agent folder" is any direct child of `<root>/<workspace_name>/`
/// that contains `.houston/agent.json` (the same marker
/// [`crate::agents_crud::list`] uses). Hidden directories (`.foo`),
/// non-directories, and symlinks pointing at missing targets are
/// skipped. Symlinked agent folders (linked external projects) ARE
/// migrated — `ensure_repo_sync` follows the symlink and operates on
/// the real target, matching create-time behavior.
pub fn migrate_ensure_agent_git_repos(root: &Path) -> CoreResult<MigrationStats> {
    let mut stats = MigrationStats::default();
    let all = workspaces::list(root)?;

    for ws in all.iter() {
        let ws_dir = root.join(&ws.name);
        if !ws_dir.is_dir() {
            continue;
        }
        stats.workspaces_scanned += 1;
        walk_workspace_agents(&ws_dir, &mut stats);
    }

    Ok(stats)
}

fn walk_workspace_agents(ws_dir: &Path, stats: &mut MigrationStats) {
    let entries = match fs::read_dir(ws_dir) {
        Ok(e) => e,
        Err(e) => {
            stats
                .errors
                .push(format!("read_dir {}: {e}", ws_dir.display()));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        // Dangling symlinks (linked-agent target deleted out-of-band):
        // skip silently — the next list() call will prune them.
        if path.is_symlink() && !path.exists() {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        // Marker: only agent directories carry `.houston/agent.json`.
        // Other top-level entries (README dumps, the user's notes,
        // future workspace-level files) must NOT be init'd.
        if !path.join(".houston").join("agent.json").exists() {
            continue;
        }
        stats.agents_scanned += 1;
        match ensure_repo_sync(&path) {
            Ok(EnsureRepoOutcome::Initialized) => {
                stats.initialized += 1;
                tracing::info!(
                    target: "houston_engine_core::agents::git_init_migration",
                    agent = %path.display(),
                    "initialized git repo for historical agent"
                );
            }
            Ok(EnsureRepoOutcome::AlreadyAGitRepo) => {
                stats.already_a_repo += 1;
            }
            Err(e) => {
                stats.errors.push(format!("{}: {e}", path.display()));
                tracing::warn!(
                    target: "houston_engine_core::agents::git_init_migration",
                    agent = %path.display(),
                    error = %e,
                    "git init migration failed for agent"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspaces::CreateWorkspace;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    /// Helper: register a workspace in `<root>/workspaces.json` so
    /// `io::read_all` finds it, and create the matching directory.
    fn register_workspace(root: &Path, name: &str) {
        crate::workspaces::create(
            root,
            CreateWorkspace {
                name: name.to_string(),
            },
        )
        .unwrap();
    }

    /// Helper: drop a minimal `.houston/agent.json` marker so the
    /// migration recognises the folder as an agent.
    fn mark_as_agent(agent_dir: &Path) {
        fs::create_dir_all(agent_dir.join(".houston")).unwrap();
        fs::write(
            agent_dir.join(".houston/agent.json"),
            r#"{"id":"a1","configId":"blank","color":null,"createdAt":"t","lastOpenedAt":null}"#,
        )
        .unwrap();
    }

    #[test]
    fn empty_root_returns_empty_stats() {
        let d = TempDir::new().unwrap();
        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(stats, MigrationStats::default());
    }

    #[test]
    fn workspace_with_no_agents_is_a_noop() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(stats.workspaces_scanned, 1);
        assert_eq!(stats.agents_scanned, 0);
        assert_eq!(stats.initialized, 0);
        assert_eq!(stats.already_a_repo, 0);
    }

    #[test]
    fn historical_agent_without_git_gets_initialized() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        let agent = d.path().join("Personal").join("HistoricalAgent");
        mark_as_agent(&agent);
        assert!(!agent.join(".git").exists(), "precondition: no .git");

        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();

        assert_eq!(stats.workspaces_scanned, 1);
        assert_eq!(stats.agents_scanned, 1);
        assert_eq!(stats.initialized, 1);
        assert_eq!(stats.already_a_repo, 0);
        assert!(stats.errors.is_empty(), "errors: {:?}", stats.errors);
        assert!(
            agent.join(".git").exists(),
            "agent dir must have .git after migration"
        );
    }

    #[test]
    fn already_initialized_agent_is_left_alone() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        let agent = d.path().join("Personal").join("ExistingRepo");
        mark_as_agent(&agent);
        // Pre-init on a non-main branch so we can prove the migration
        // doesn't touch the existing state.
        let out = StdCommand::new("git")
            .args(["init", "-q", "-b", "trunk"])
            .current_dir(&agent)
            .output()
            .expect("spawn git");
        assert!(out.status.success());

        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();

        assert_eq!(stats.initialized, 0);
        assert_eq!(stats.already_a_repo, 1);
        let head = fs::read_to_string(agent.join(".git/HEAD")).unwrap();
        assert!(
            head.trim().ends_with("refs/heads/trunk"),
            "branch must be preserved, got: {head:?}"
        );
    }

    #[test]
    fn dot_prefixed_dirs_are_skipped() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        // A `.config` sibling inside the workspace shouldn't be treated
        // as an agent even if it happens to carry a `.houston/agent.json`.
        let pseudo_agent = d.path().join("Personal").join(".config");
        mark_as_agent(&pseudo_agent);

        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(stats.agents_scanned, 0);
        assert!(!pseudo_agent.join(".git").exists());
    }

    #[test]
    fn non_agent_dirs_are_skipped() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        // A plain folder without `.houston/agent.json` — could be the
        // user's notes or a stray dir. Must not be init'd.
        let plain = d.path().join("Personal").join("just-a-folder");
        fs::create_dir_all(&plain).unwrap();
        fs::write(plain.join("notes.txt"), "hi").unwrap();

        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(stats.agents_scanned, 0);
        assert!(!plain.join(".git").exists());
    }

    #[test]
    fn migration_is_idempotent_across_runs() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        let a1 = d.path().join("Personal").join("FirstAgent");
        let a2 = d.path().join("Personal").join("SecondAgent");
        mark_as_agent(&a1);
        mark_as_agent(&a2);

        let first = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(first.initialized, 2);
        assert_eq!(first.already_a_repo, 0);

        let second = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(second.initialized, 0);
        assert_eq!(second.already_a_repo, 2);

        let third = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(third.initialized, 0);
        assert_eq!(third.already_a_repo, 2);
    }

    #[test]
    fn mixed_workspace_is_handled_per_agent() {
        let d = TempDir::new().unwrap();
        register_workspace(d.path(), "Personal");
        let needs_init = d.path().join("Personal").join("NeedsInit");
        let already_done = d.path().join("Personal").join("AlreadyDone");
        mark_as_agent(&needs_init);
        mark_as_agent(&already_done);
        // Pre-init only the second agent.
        let out = StdCommand::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(&already_done)
            .output()
            .expect("spawn git");
        assert!(out.status.success());

        let stats = migrate_ensure_agent_git_repos(d.path()).unwrap();
        assert_eq!(stats.agents_scanned, 2);
        assert_eq!(stats.initialized, 1);
        assert_eq!(stats.already_a_repo, 1);
        assert!(needs_init.join(".git").exists());
        assert!(already_done.join(".git").exists());
    }
}
