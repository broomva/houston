//! Corpus assembly + atomic staging.

use crate::context_bootstrap::limits::{truncate_chars, MAX_CORPUS_CHARS, MAX_DOC_CHARS};
use crate::context_bootstrap::CorpusDoc;
use crate::error::CoreResult;
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Join harvested docs into one markdown corpus, bounded by the total budget.
/// Returns the corpus and whether any cap clipped content.
pub(crate) fn assemble(docs: &[CorpusDoc]) -> (String, bool) {
    let mut out = String::new();
    let mut truncated = false;

    for doc in docs {
        let used = out.chars().count();
        if used >= MAX_CORPUS_CHARS {
            truncated = true;
            break;
        }
        let header = format!("\n\n## [{}] {}\n\n", doc.source_kind.human_label(), doc.label);
        let remaining = MAX_CORPUS_CHARS
            .saturating_sub(used)
            .saturating_sub(header.chars().count());
        let budget = remaining.min(MAX_DOC_CHARS);
        if budget == 0 {
            truncated = true;
            break;
        }
        let (body, clipped) = truncate_chars(doc.text.trim(), budget);
        if clipped {
            truncated = true;
        }
        out.push_str(&header);
        out.push_str(&body);
    }

    (out.trim_start().to_string(), truncated)
}

/// Write `content` to `path` atomically (unique temp + rename), matching the
/// `.houston/` write discipline.
pub(crate) fn write_atomic(path: &Path, content: &str) -> CoreResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(".tmp-{}", Uuid::new_v4()));
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_bootstrap::ImportSourceKind;

    fn doc(text: &str) -> CorpusDoc {
        CorpusDoc {
            source_kind: ImportSourceKind::LocalFolder,
            label: "x.md".into(),
            text: text.into(),
        }
    }

    #[test]
    fn assemble_includes_label_and_body() {
        let (out, truncated) = assemble(&[doc("Acme Corp, B2B fintech.")]);
        assert!(out.contains("local folder"));
        assert!(out.contains("x.md"));
        assert!(out.contains("Acme Corp, B2B fintech."));
        assert!(!truncated);
    }

    #[test]
    fn assemble_flags_truncation_past_budget() {
        let big = "a".repeat(MAX_CORPUS_CHARS + 10_000);
        let (out, truncated) = assemble(&[doc(&big)]);
        assert!(truncated);
        assert!(out.chars().count() <= MAX_CORPUS_CHARS);
    }

    #[test]
    fn write_atomic_creates_file() {
        let d = tempfile::TempDir::new().unwrap();
        let p = d.path().join("corpus.md");
        write_atomic(&p, "hi").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "hi");
    }
}
