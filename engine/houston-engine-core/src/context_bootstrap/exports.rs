//! ChatGPT / Claude.ai data-export parsing.
//!
//! Both ship a `conversations.json` whose exact shape drifts across versions, so
//! rather than bind a brittle struct we harvest message text from a small set of
//! relevant keys (`parts`, `text`, `title`, `name`) anywhere in each
//! conversation's JSON. One corpus doc per conversation.

use crate::context_bootstrap::limits::{truncate_chars, MAX_CONVERSATIONS, MAX_DOC_CHARS};
use crate::context_bootstrap::redact::redact_secrets;
use crate::context_bootstrap::{CorpusDoc, ImportSourceKind, SkippedDoc};
use crate::error::{CoreError, CoreResult};
use serde_json::Value;
use std::path::Path;

/// Parse an export file into one doc per conversation. A missing file or invalid
/// JSON is a hard error (surfaced as a toast); an empty conversation is skipped.
pub(crate) fn parse_json_export(
    path: &Path,
    kind: ImportSourceKind,
    keys: &[&str],
    docs: &mut Vec<CorpusDoc>,
    skipped: &mut Vec<SkippedDoc>,
) -> CoreResult<()> {
    if !path.is_file() {
        return Err(CoreError::BadRequest(format!(
            "not an export file: {}",
            path.display()
        )));
    }
    let raw = std::fs::read_to_string(path)?;
    let root: Value = serde_json::from_str(&raw)
        .map_err(|e| CoreError::BadRequest(format!("export is not valid JSON: {e}")))?;

    let conversations = conversation_list(&root);
    if conversations.is_empty() {
        return Err(CoreError::BadRequest(
            "export contained no conversations".into(),
        ));
    }

    for (i, convo) in conversations.iter().enumerate() {
        if i >= MAX_CONVERSATIONS {
            skipped.push(SkippedDoc {
                path: path.display().to_string(),
                reason: format!(
                    "conversation limit reached ({MAX_CONVERSATIONS}); remaining not read"
                ),
            });
            break;
        }
        let mut buf = String::new();
        harvest(convo, keys, &mut buf);
        let trimmed = buf.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (text, _) = truncate_chars(trimmed, MAX_DOC_CHARS);
        docs.push(CorpusDoc {
            source_kind: kind,
            label: conversation_label(convo, i),
            text: redact_secrets(&text),
        });
    }
    Ok(())
}

/// Locate the conversation array: a top-level array, a `conversations` field, or
/// the whole value treated as one conversation.
fn conversation_list(root: &Value) -> Vec<Value> {
    if let Some(arr) = root.as_array() {
        return arr.clone();
    }
    if let Some(arr) = root.get("conversations").and_then(|c| c.as_array()) {
        return arr.clone();
    }
    vec![root.clone()]
}

fn conversation_label(convo: &Value, index: usize) -> String {
    for key in ["title", "name"] {
        if let Some(s) = convo.get(key).and_then(|v| v.as_str()) {
            if !s.trim().is_empty() {
                return s.trim().to_string();
            }
        }
    }
    format!("conversation {}", index + 1)
}

/// Recursively collect strings stored under any key in `keys`.
fn harvest(value: &Value, keys: &[&str], out: &mut String) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if keys.contains(&key.as_str()) {
                    collect_strings(child, out);
                }
                harvest(child, keys, out);
            }
        }
        Value::Array(arr) => {
            for child in arr {
                harvest(child, keys, out);
            }
        }
        _ => {}
    }
}

/// Append every non-empty string leaf in `value` (strings, arrays of strings,
/// and `{type, text}` content blocks).
fn collect_strings(value: &Value, out: &mut String) {
    match value {
        Value::String(s) => {
            if !s.trim().is_empty() {
                out.push_str(s);
                out.push('\n');
            }
        }
        Value::Array(arr) => arr.iter().for_each(|c| collect_strings(c, out)),
        Value::Object(map) => {
            if let Some(text) = map.get("text") {
                collect_strings(text, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn parse(json: &str, kind: ImportSourceKind, keys: &[&str]) -> Vec<CorpusDoc> {
        let d = TempDir::new().unwrap();
        let p = d.path().join("conversations.json");
        fs::write(&p, json).unwrap();
        let mut docs = Vec::new();
        let mut skipped = Vec::new();
        parse_json_export(&p, kind, keys, &mut docs, &mut skipped).unwrap();
        docs
    }

    #[test]
    fn chatgpt_export_harvests_parts_and_title() {
        let json = r#"[
          {"title":"Sales planning","mapping":{
            "n1":{"message":{"author":{"role":"user"},"content":{"content_type":"text","parts":["Help me plan outreach to B2B fintech leads"]}}},
            "n2":{"message":{"author":{"role":"assistant"},"content":{"content_type":"text","parts":["Sure, here is a plan"]}}}
          }}
        ]"#;
        let docs = parse(json, ImportSourceKind::ChatGptExport, &["parts", "title", "text", "name"]);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].label, "Sales planning");
        assert!(docs[0].text.contains("B2B fintech leads"));
        assert!(docs[0].text.contains("here is a plan"));
    }

    #[test]
    fn claude_ai_export_harvests_text_and_content_blocks() {
        let json = r#"[
          {"name":"Recruiting","chat_messages":[
            {"sender":"human","text":"I am hiring two engineers"},
            {"sender":"assistant","content":[{"type":"text","text":"Great, let us define the roles"}]}
          ]}
        ]"#;
        let docs = parse(json, ImportSourceKind::ClaudeAiExport, &["text", "name", "title", "parts"]);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].label, "Recruiting");
        assert!(docs[0].text.contains("hiring two engineers"));
        assert!(docs[0].text.contains("define the roles"));
    }

    #[test]
    fn invalid_json_is_hard_error() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("conversations.json");
        fs::write(&p, "{not json").unwrap();
        let err = parse_json_export(
            &p,
            ImportSourceKind::ChatGptExport,
            &["parts"],
            &mut Vec::new(),
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::BadRequest(_)));
    }

    #[test]
    fn empty_conversations_skipped_not_errored() {
        let json = r#"[{"title":"Empty","chat_messages":[]}]"#;
        let docs = parse(json, ImportSourceKind::ClaudeAiExport, &["text", "name"]);
        assert!(docs.is_empty());
    }
}
