//! Bounding constants + helpers for corpus ingestion. Keep the corpus small
//! enough to fit a synthesis prompt and to never OOM on a pathological folder.

/// Max files read per folder/vault/home source.
pub const MAX_FILES: usize = 400;
/// Files larger than this are skipped (almost never prose).
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// A whole ChatGPT/Claude export JSON larger than this is rejected before it is
/// read into memory — real exports can be hundreds of MB and would OOM the
/// engine (the file is held as a String AND a serde Value tree).
pub const MAX_EXPORT_BYTES: u64 = 200 * 1024 * 1024;
/// A single document is truncated to this many chars before it joins the corpus.
pub const MAX_DOC_CHARS: usize = 40_000;
/// Total assembled corpus budget (≈ the synthesis prompt ceiling).
pub const MAX_CORPUS_CHARS: usize = 400_000;
/// Max conversations harvested from one export file.
pub const MAX_CONVERSATIONS: usize = 200;

/// Text-file extensions ingested from a generic local folder.
pub const ALLOWED_TEXT_EXTS: &[&str] = &["md", "markdown", "txt", "text", "rst", "org", "csv"];
/// Markdown-only extensions (vault + Claude memory).
pub const MARKDOWN_EXTS: &[&str] = &["md", "markdown"];
/// Directory names pruned during a walk (noise, tooling, VCS).
pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
    "vendor",
];

/// Truncate to `max` chars on a char boundary. Returns whether it clipped.
pub fn truncate_chars(s: &str, max: usize) -> (String, bool) {
    if s.chars().count() <= max {
        (s.to_string(), false)
    } else {
        (s.chars().take(max).collect(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_strings() {
        let (out, clipped) = truncate_chars("hello", 10);
        assert_eq!(out, "hello");
        assert!(!clipped);
    }

    #[test]
    fn truncate_clips_long_strings_on_char_boundary() {
        let (out, clipped) = truncate_chars("áéíóú", 3);
        assert_eq!(out, "áéí");
        assert!(clipped);
    }
}
