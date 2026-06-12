//! Built-in skills that ship with every Houston agent.
//!
//! Unlike Store-packaged skills (copied only into agents installed from a Store
//! package) these are seeded into **every** agent — built-in, blank, imported,
//! or Store — via `migrate_agent_data`, which runs on create and on load. The
//! markdown is embedded at compile time so it ships inside the engine binary
//! with no on-disk dependency.

use std::path::Path;

use crate::{write_file_atomic, Result};

/// `write-my-job-description` — the guided instruction-writer the Job
/// Description screen's "Help me write this" button invokes.
pub const WRITE_MY_JOB_DESCRIPTION: &str =
    include_str!("../assets/skills/write-my-job-description/SKILL.md");

/// `(slug, SKILL.md content)` for every built-in skill. The slug must match the
/// `name:` in the embedded frontmatter and the directory it is written to.
pub const ALL: &[(&str, &str)] = &[("write-my-job-description", WRITE_MY_JOB_DESCRIPTION)];

/// Seed the embedded built-in skills under `.agents/skills/<slug>/SKILL.md`.
///
/// Idempotent and **non-destructive**: a skill is written only when its
/// `SKILL.md` is absent. Once present we never overwrite it — the user (or the
/// agent) may have edited or intentionally deleted it, and their copy wins, the
/// same contract Store-skill sync uses ("add only, never clobber"). The
/// `.claude/skills/<slug>` discovery symlink is created lazily by the engine on
/// the next `list_skills`, so this step only needs to drop the source file.
pub fn seed_builtin_skills(agent_root: &Path) -> Result<()> {
    for (slug, content) in ALL {
        let rel = format!(".agents/skills/{slug}/SKILL.md");
        if agent_root.join(&rel).exists() {
            continue;
        }
        write_file_atomic(agent_root, &rel, content)?;
        tracing::info!(
            agent_root = %agent_root.display(),
            skill = slug,
            "seeded built-in skill"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn skill_path(root: &Path, slug: &str) -> std::path::PathBuf {
        root.join(".agents/skills").join(slug).join("SKILL.md")
    }

    #[test]
    fn embedded_skills_parse_and_match_their_slug() {
        for (slug, content) in ALL {
            let (summary, body) = houston_skills::format::parse_content(content)
                .unwrap_or_else(|e| panic!("built-in skill {slug} has invalid frontmatter: {e}"));
            assert_eq!(
                &summary.name, slug,
                "frontmatter name must match the directory slug"
            );
            assert!(
                !summary.description.is_empty(),
                "{slug} needs a description (drives tool matching)"
            );
            // Built-ins must not clutter the empty-state showcase.
            assert!(!summary.featured, "{slug} must not be featured");
            assert!(!body.trim().is_empty(), "{slug} needs a procedure body");
        }
    }

    #[test]
    fn seed_creates_each_builtin_skill() {
        let tmp = TempDir::new().unwrap();
        seed_builtin_skills(tmp.path()).unwrap();
        for (slug, content) in ALL {
            let path = skill_path(tmp.path(), slug);
            assert!(path.exists(), "{slug} should be seeded");
            assert_eq!(&std::fs::read_to_string(&path).unwrap(), content);
        }
    }

    #[test]
    fn seed_is_idempotent_and_non_destructive() {
        let tmp = TempDir::new().unwrap();
        let (slug, _) = ALL[0];
        let path = skill_path(tmp.path(), slug);

        // Simulate a user (or the agent) having edited the seeded skill.
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let edited =
            "---\nname: write-my-job-description\ndescription: my edited copy\n---\n\nMine.\n";
        std::fs::write(&path, edited).unwrap();

        // Re-seeding must not clobber the edited copy.
        seed_builtin_skills(tmp.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
    }

    #[test]
    fn migrate_agent_data_seeds_builtin_skills() {
        let tmp = TempDir::new().unwrap();
        crate::migrate_agent_data(tmp.path()).unwrap();
        let (slug, _) = ALL[0];
        assert!(
            skill_path(tmp.path(), slug).exists(),
            "migration should seed built-in skills"
        );
    }
}
