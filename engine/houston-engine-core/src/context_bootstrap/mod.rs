//! Context bootstrap — proactive workspace/user context enablement.
//!
//! Phase 1 ingests a corpus from the user's existing knowledge (a folder,
//! `~/.claude` memory, a notes vault, or a ChatGPT/Claude export), bounds and
//! redacts it, and stages it on disk. A later synthesis step (see
//! `synthesize.rs`) runs that corpus through the user's own provider CLI to
//! draft `USER.md` + `WORKSPACE.md` and a list of residual questions.
//!
//! Nothing here writes the final context files — the staged corpus is input to
//! synthesis, and synthesis output is reviewed by the user before the existing
//! `PUT /v1/workspaces/:id/context` route persists it.
//!
//! All processing is local. Secrets are redacted before staging. Per-file
//! problems are reported in `ImportSummary.skipped` (never silently dropped); a
//! whole-source failure is a hard `CoreError` so the UI can surface a toast.

mod corpus;
mod draft;
mod exports;
mod limits;
mod parsers;
mod redact;
mod synthesize;

pub use draft::{ContextDraft, QuestionKind, ResidualQuestion, Slot};
pub use synthesize::synthesize;

use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Where a corpus is gathered from. Type-safe per the engine convention
/// (domain concepts are enums with `Display` + `FromStr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportSourceKind {
    /// A folder the user picked (notes, docs, READMEs). Text-extension allowlist.
    LocalFolder,
    /// The user's `~/.claude` memory: top-level `CLAUDE.md` + `projects/**/*.md`.
    ClaudeHome,
    /// An Obsidian-style markdown vault (`.md` only, `[[wikilinks]]` preserved).
    ObsidianVault,
    /// A ChatGPT data export (`conversations.json`).
    ChatGptExport,
    /// A Claude.ai data export (`conversations.json`).
    ClaudeAiExport,
}

impl ImportSourceKind {
    /// The camelCase wire token (matches serde + the TS discriminated union).
    pub fn wire(&self) -> &'static str {
        match self {
            Self::LocalFolder => "localFolder",
            Self::ClaudeHome => "claudeHome",
            Self::ObsidianVault => "obsidianVault",
            Self::ChatGptExport => "chatGptExport",
            Self::ClaudeAiExport => "claudeAiExport",
        }
    }

    /// Human-readable label used in corpus section headers.
    pub fn human_label(&self) -> &'static str {
        match self {
            Self::LocalFolder => "local folder",
            Self::ClaudeHome => "Claude memory",
            Self::ObsidianVault => "notes vault",
            Self::ChatGptExport => "ChatGPT export",
            Self::ClaudeAiExport => "Claude export",
        }
    }
}

impl std::fmt::Display for ImportSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.wire())
    }
}

impl std::str::FromStr for ImportSourceKind {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "localFolder" => Ok(Self::LocalFolder),
            "claudeHome" => Ok(Self::ClaudeHome),
            "obsidianVault" => Ok(Self::ObsidianVault),
            "chatGptExport" => Ok(Self::ChatGptExport),
            "claudeAiExport" => Ok(Self::ClaudeAiExport),
            other => Err(CoreError::BadRequest(format!(
                "unknown import source kind: {other}"
            ))),
        }
    }
}

/// One source the user asked to import. `path` is a folder (folder/vault/home)
/// or a file (the export JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    pub kind: ImportSourceKind,
    pub path: PathBuf,
}

/// A file/conversation that could not be ingested, with the reason. Surfaced to
/// the user — never swallowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedDoc {
    pub path: String,
    pub reason: String,
}

/// Result of an import: what landed in the staged corpus and what didn't.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    /// Number of documents that made it into the corpus.
    pub docs: usize,
    /// Byte size of the assembled corpus on disk.
    pub bytes: u64,
    /// Files/conversations skipped, with reasons.
    pub skipped: Vec<SkippedDoc>,
    /// True when a cap (per-doc, total budget, or file count) clipped content.
    pub truncated: bool,
}

/// Internal: one harvested document before corpus assembly.
pub(crate) struct CorpusDoc {
    pub source_kind: ImportSourceKind,
    /// Relative path or conversation title — a short human-readable label.
    pub label: String,
    pub text: String,
}

/// `<ws>/.houston/context-import` — where the staged corpus lives.
pub fn staging_dir(ws_dir: &Path) -> PathBuf {
    ws_dir.join(".houston").join("context-import")
}

/// Ingest every source into one bounded, redacted corpus staged under the
/// workspace. Returns a summary the UI shows before synthesis.
pub fn ingest(ws_dir: &Path, sources: &[ImportSource]) -> CoreResult<ImportSummary> {
    if sources.is_empty() {
        return Err(CoreError::BadRequest("no import sources provided".into()));
    }

    let mut docs: Vec<CorpusDoc> = Vec::new();
    let mut skipped: Vec<SkippedDoc> = Vec::new();
    for src in sources {
        parsers::parse_source(src, &mut docs, &mut skipped)?;
    }

    let (assembled, truncated) = corpus::assemble(&docs);
    let dir = staging_dir(ws_dir);
    fs::create_dir_all(&dir)?;
    corpus::write_atomic(&dir.join("corpus.md"), &assembled)?;

    let summary = ImportSummary {
        docs: docs.len(),
        bytes: assembled.len() as u64,
        skipped,
        truncated,
    };
    corpus::write_atomic(
        &dir.join("summary.json"),
        &serde_json::to_string_pretty(&summary)?,
    )?;
    Ok(summary)
}

/// Read the staged corpus (input to synthesis). `NotFound` if import never ran.
pub fn read_corpus(ws_dir: &Path) -> CoreResult<String> {
    let path = staging_dir(ws_dir).join("corpus.md");
    fs::read_to_string(&path)
        .map_err(|_| CoreError::NotFound("no imported context yet; run import first".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tempfile::TempDir;

    #[test]
    fn kind_wire_roundtrips_through_fromstr() {
        for k in [
            ImportSourceKind::LocalFolder,
            ImportSourceKind::ClaudeHome,
            ImportSourceKind::ObsidianVault,
            ImportSourceKind::ChatGptExport,
            ImportSourceKind::ClaudeAiExport,
        ] {
            assert_eq!(ImportSourceKind::from_str(k.wire()).unwrap(), k);
        }
        assert!(ImportSourceKind::from_str("nope").is_err());
    }

    #[test]
    fn kind_serializes_as_camel_case() {
        let json = serde_json::to_string(&ImportSourceKind::ChatGptExport).unwrap();
        assert_eq!(json, "\"chatGptExport\"");
    }

    #[test]
    fn ingest_rejects_empty_sources() {
        let d = TempDir::new().unwrap();
        let err = ingest(d.path(), &[]).unwrap_err();
        assert!(matches!(err, CoreError::BadRequest(_)));
    }

    #[test]
    fn ingest_stages_corpus_and_summary() {
        let ws = TempDir::new().unwrap();
        let folder = TempDir::new().unwrap();
        fs::write(folder.path().join("notes.md"), "I run a B2B fintech.").unwrap();
        fs::write(folder.path().join("photo.png"), [0u8, 1, 2]).unwrap();

        let summary = ingest(
            ws.path(),
            &[ImportSource {
                kind: ImportSourceKind::LocalFolder,
                path: folder.path().to_path_buf(),
            }],
        )
        .unwrap();

        assert_eq!(
            summary.docs, 1,
            "only the .md counts; .png is not an allowed ext"
        );
        assert!(summary.bytes > 0);

        let staged = staging_dir(ws.path());
        assert!(staged.join("corpus.md").exists());
        assert!(staged.join("summary.json").exists());

        let corpus = read_corpus(ws.path()).unwrap();
        assert!(corpus.contains("B2B fintech"));
        assert!(corpus.contains("notes.md"));
    }

    #[test]
    fn read_corpus_not_found_before_import() {
        let ws = TempDir::new().unwrap();
        assert!(matches!(
            read_corpus(ws.path()).unwrap_err(),
            CoreError::NotFound(_)
        ));
    }
}
