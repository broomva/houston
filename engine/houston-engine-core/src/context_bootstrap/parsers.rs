//! Per-source corpus parsers: local folder, notes vault, and `~/.claude`
//! memory. Export-file parsing lives in `exports.rs`.

use crate::context_bootstrap::exports::parse_json_export;
use crate::context_bootstrap::limits::{
    ALLOWED_TEXT_EXTS, MARKDOWN_EXTS, MAX_FILES, MAX_FILE_BYTES, SKIP_DIRS,
};
use crate::context_bootstrap::redact::redact_secrets;
use crate::context_bootstrap::{CorpusDoc, ImportSource, ImportSourceKind, SkippedDoc};
use crate::error::{CoreError, CoreResult};
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

/// Dispatch one source to its parser. A whole-source failure (missing path,
/// unparseable export) is a hard error so the route surfaces a toast; per-file
/// problems land in `skipped`.
pub(crate) fn parse_source(
    src: &ImportSource,
    docs: &mut Vec<CorpusDoc>,
    skipped: &mut Vec<SkippedDoc>,
) -> CoreResult<()> {
    // Expand a leading `~` once for every kind so REST callers can submit
    // `~/...` paths verbatim (the `paths.rs` contract). The desktop picker
    // already returns absolute paths, so this is a no-op there.
    let path = crate::paths::expand_tilde(&src.path);
    match src.kind {
        ImportSourceKind::LocalFolder => walk_text_folder(
            &path,
            ImportSourceKind::LocalFolder,
            ALLOWED_TEXT_EXTS,
            docs,
            skipped,
        ),
        ImportSourceKind::ObsidianVault => walk_text_folder(
            &path,
            ImportSourceKind::ObsidianVault,
            MARKDOWN_EXTS,
            docs,
            skipped,
        ),
        ImportSourceKind::ClaudeHome => parse_claude_home(&path, docs, skipped),
        // Note: "name"/"title" are NOT harvested into the body — the
        // conversation label reads them directly. Harvesting "name" would slurp
        // every author.name as message text.
        ImportSourceKind::ChatGptExport => parse_json_export(
            &path,
            ImportSourceKind::ChatGptExport,
            &["parts", "text"],
            docs,
            skipped,
        ),
        ImportSourceKind::ClaudeAiExport => parse_json_export(
            &path,
            ImportSourceKind::ClaudeAiExport,
            &["text", "parts"],
            docs,
            skipped,
        ),
    }
}

/// `~/.claude` memory: the top-level `CLAUDE.md` plus markdown under
/// `projects/**` (memory files). Skill/plugin docs and `.jsonl` transcripts are
/// intentionally excluded — they are tooling, not the user's context.
fn parse_claude_home(
    path: &Path,
    docs: &mut Vec<CorpusDoc>,
    skipped: &mut Vec<SkippedDoc>,
) -> CoreResult<()> {
    // `path` is already tilde-expanded by `parse_source`.
    let home = path;
    if !home.is_dir() {
        return Err(CoreError::BadRequest(format!(
            "not a Claude home folder: {}",
            home.display()
        )));
    }

    let top = home.join("CLAUDE.md");
    if top.is_file() {
        match read_text_capped(&top) {
            Ok(text) => docs.push(CorpusDoc {
                source_kind: ImportSourceKind::ClaudeHome,
                label: "CLAUDE.md".into(),
                text: redact_secrets(&text),
            }),
            Err(reason) => skipped.push(SkippedDoc {
                path: top.display().to_string(),
                reason,
            }),
        }
    }

    let projects = home.join("projects");
    if projects.is_dir() {
        walk_text_folder(
            &projects,
            ImportSourceKind::ClaudeHome,
            MARKDOWN_EXTS,
            docs,
            skipped,
        )?;
    }
    Ok(())
}

/// Walk a folder, ingesting files whose extension is in `allowed`. Prunes
/// hidden and noisy directories. Records per-file skips; caps at `MAX_FILES`.
fn walk_text_folder(
    root: &Path,
    kind: ImportSourceKind,
    allowed: &[&str],
    docs: &mut Vec<CorpusDoc>,
    skipped: &mut Vec<SkippedDoc>,
) -> CoreResult<()> {
    if !root.is_dir() {
        return Err(CoreError::BadRequest(format!(
            "not a folder: {}",
            root.display()
        )));
    }

    let mut count = 0usize;
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_pruned_dir(e));

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                skipped.push(SkippedDoc {
                    path: e
                        .path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    reason: e.to_string(),
                });
                continue;
            }
        };
        if !entry.file_type().is_file() || !has_allowed_ext(entry.path(), allowed) {
            continue;
        }
        if count >= MAX_FILES {
            skipped.push(SkippedDoc {
                path: root.display().to_string(),
                reason: format!("file limit reached ({MAX_FILES}); remaining files not read"),
            });
            break;
        }
        match read_text_capped(entry.path()) {
            Ok(text) => {
                let label = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .display()
                    .to_string();
                docs.push(CorpusDoc {
                    source_kind: kind,
                    label,
                    text: redact_secrets(&text),
                });
                count += 1;
            }
            Err(reason) => skipped.push(SkippedDoc {
                path: entry.path().display().to_string(),
                reason,
            }),
        }
    }
    Ok(())
}

/// Prune hidden directories (depth > 0) and known-noise dirs. The root itself is
/// never pruned, so a hidden root like `~/.claude` is still walked.
fn is_pruned_dir(e: &DirEntry) -> bool {
    if e.depth() == 0 || !e.file_type().is_dir() {
        return false;
    }
    match e.file_name().to_str() {
        Some(name) => name.starts_with('.') || SKIP_DIRS.contains(&name),
        None => false,
    }
}

fn has_allowed_ext(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| allowed.iter().any(|a| a.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

/// Read a text file, rejecting oversized or binary files (with a reason).
fn read_text_capped(path: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("file too large ({} bytes)", meta.len()));
    }
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    if raw.contains(&0) {
        return Err("binary file skipped".into());
    }
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn collect(src: &ImportSource) -> (Vec<CorpusDoc>, Vec<SkippedDoc>) {
        let mut docs = Vec::new();
        let mut skipped = Vec::new();
        parse_source(src, &mut docs, &mut skipped).unwrap();
        (docs, skipped)
    }

    #[test]
    fn local_folder_allowlist_and_pruning() {
        let d = TempDir::new().unwrap();
        fs::write(d.path().join("a.md"), "alpha notes").unwrap();
        fs::write(d.path().join("b.txt"), "beta notes").unwrap();
        fs::write(d.path().join("c.png"), [0u8, 1, 2]).unwrap();
        fs::create_dir_all(d.path().join("node_modules")).unwrap();
        fs::write(d.path().join("node_modules/dep.md"), "should be pruned").unwrap();
        fs::create_dir_all(d.path().join(".hidden")).unwrap();
        fs::write(d.path().join(".hidden/secret.md"), "should be pruned").unwrap();

        let (docs, _) = collect(&ImportSource {
            kind: ImportSourceKind::LocalFolder,
            path: d.path().to_path_buf(),
        });
        let labels: Vec<&str> = docs.iter().map(|x| x.label.as_str()).collect();
        assert!(labels.contains(&"a.md"));
        assert!(labels.contains(&"b.txt"));
        assert_eq!(docs.len(), 2, "png + pruned dirs excluded");
    }

    #[test]
    fn obsidian_vault_is_markdown_only() {
        let d = TempDir::new().unwrap();
        fs::write(d.path().join("note.md"), "links to [[other]]").unwrap();
        fs::write(d.path().join("data.csv"), "a,b,c").unwrap();
        fs::create_dir_all(d.path().join(".obsidian")).unwrap();
        fs::write(d.path().join(".obsidian/app.json"), "{}").unwrap();

        let (docs, _) = collect(&ImportSource {
            kind: ImportSourceKind::ObsidianVault,
            path: d.path().to_path_buf(),
        });
        assert_eq!(docs.len(), 1);
        assert!(docs[0].text.contains("[[other]]"), "wikilinks preserved");
    }

    #[test]
    fn claude_home_reads_claude_md_and_project_memory_only() {
        let d = TempDir::new().unwrap();
        fs::write(d.path().join("CLAUDE.md"), "global memory").unwrap();
        let mem = d.path().join("projects/proj/memory");
        fs::create_dir_all(&mem).unwrap();
        fs::write(mem.join("MEMORY.md"), "project memory").unwrap();
        // skills should be ignored — tooling, not user context
        let skills = d.path().join("skills/foo");
        fs::create_dir_all(&skills).unwrap();
        fs::write(skills.join("SKILL.md"), "a skill doc").unwrap();

        let (docs, _) = collect(&ImportSource {
            kind: ImportSourceKind::ClaudeHome,
            path: d.path().to_path_buf(),
        });
        let joined: String = docs.iter().map(|x| x.text.clone()).collect();
        assert!(joined.contains("global memory"));
        assert!(joined.contains("project memory"));
        assert!(!joined.contains("a skill doc"), "skills excluded");
    }

    #[test]
    fn missing_folder_is_hard_error() {
        let err = parse_source(
            &ImportSource {
                kind: ImportSourceKind::LocalFolder,
                path: "/no/such/folder/xyz".into(),
            },
            &mut Vec::new(),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::BadRequest(_)));
    }

    #[test]
    fn secrets_redacted_during_ingest() {
        let d = TempDir::new().unwrap();
        fs::write(d.path().join("n.md"), "token=abcd1234efgh5678ijkl").unwrap();
        let (docs, _) = collect(&ImportSource {
            kind: ImportSourceKind::LocalFolder,
            path: d.path().to_path_buf(),
        });
        assert!(docs[0].text.contains("[REDACTED]"));
        assert!(!docs[0].text.contains("abcd1234efgh5678ijkl"));
    }
}
