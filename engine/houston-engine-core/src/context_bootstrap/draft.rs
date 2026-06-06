//! The synthesis output: a draft `USER.md` + `WORKSPACE.md` plus the residual
//! questions the agent still needs answered. Parsing is lenient — a malformed
//! single question is dropped, not fatal — but a wholly empty result is an error
//! (no silent empty draft).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Which context file a question (or fact) belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Slot {
    User,
    Workspace,
}

/// Whether a residual question asks for a fact, or asks the user to point the
/// agent at richer source material (which loops back into import).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionKind {
    /// A missing fact ("What is your role?").
    Content,
    /// "Where do you keep your best notes?" — answer is a path to re-import.
    SourceHint,
}

impl std::str::FromStr for Slot {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "user" => Ok(Self::User),
            "workspace" => Ok(Self::Workspace),
            _ => Err(()),
        }
    }
}

impl std::str::FromStr for QuestionKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "content" => Ok(Self::Content),
            "sourceHint" => Ok(Self::SourceHint),
            _ => Err(()),
        }
    }
}

/// One question the agent asks the user to fill a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualQuestion {
    pub id: String,
    pub prompt: String,
    pub slot: Slot,
    pub kind: QuestionKind,
}

/// The reviewable draft handed back to the UI. Nothing here is persisted until
/// the user approves and the existing `PUT context` route writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDraft {
    pub user: String,
    pub workspace: String,
    pub questions: Vec<ResidualQuestion>,
}

/// Parse a model's synthesis output into a `ContextDraft`. Tolerates markdown
/// fences and prose around the JSON object.
pub(crate) fn parse_draft(raw: &str) -> Result<ContextDraft, String> {
    let value = parse_json_lenient(&strip_fences(raw))?;

    let user = value
        .get("user")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let workspace = value
        .get("workspace")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let questions: Vec<ResidualQuestion> = value
        .get("questions")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter_map(|(i, q)| parse_question(q, i))
                .collect()
        })
        .unwrap_or_default();

    if user.is_empty() && workspace.is_empty() && questions.is_empty() {
        return Err("synthesis produced no usable draft".to_string());
    }
    Ok(ContextDraft {
        user,
        workspace,
        questions,
    })
}

fn parse_question(q: &Value, index: usize) -> Option<ResidualQuestion> {
    let prompt = q.get("prompt").and_then(Value::as_str)?.trim().to_string();
    if prompt.is_empty() {
        return None;
    }
    let id = q
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("q{}", index + 1));
    let slot = q
        .get("slot")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .unwrap_or(Slot::User);
    let kind = q
        .get("kind")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .unwrap_or(QuestionKind::Content);
    Some(ResidualQuestion {
        id,
        prompt,
        slot,
        kind,
    })
}

fn strip_fences(raw: &str) -> String {
    raw.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
}

fn parse_json_lenient(s: &str) -> Result<Value, String> {
    if let Ok(v) = serde_json::from_str::<Value>(s) {
        return Ok(v);
    }
    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        if end > start {
            if let Ok(v) = serde_json::from_str::<Value>(&s[start..=end]) {
                return Ok(v);
            }
        }
    }
    Err("synthesis output was not valid JSON".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_draft() {
        let raw = r#"{
          "user": "Juan, head of sales at a B2B fintech.",
          "workspace": "Acme Corp, sells payment APIs to LATAM SMBs.",
          "questions": [
            {"id":"goals","prompt":"What are your goals this quarter?","slot":"user","kind":"content"},
            {"id":"notes","prompt":"Where do you keep your best account notes?","slot":"workspace","kind":"sourceHint"}
          ]
        }"#;
        let d = parse_draft(raw).unwrap();
        assert!(d.user.contains("head of sales"));
        assert!(d.workspace.contains("Acme Corp"));
        assert_eq!(d.questions.len(), 2);
        assert_eq!(d.questions[1].kind, QuestionKind::SourceHint);
        assert_eq!(d.questions[1].slot, Slot::Workspace);
    }

    #[test]
    fn strips_fences_and_prose() {
        let raw = "Here is the draft:\n```json\n{\"user\":\"X\",\"workspace\":\"\",\"questions\":[]}\n```\nDone.";
        let d = parse_draft(raw).unwrap();
        assert_eq!(d.user, "X");
    }

    #[test]
    fn drops_malformed_question_but_keeps_valid() {
        let raw = r#"{"user":"X","workspace":"","questions":[
            {"slot":"user","kind":"content"},
            {"prompt":"Real question?","slot":"bogus","kind":"alsobogus"}
        ]}"#;
        let d = parse_draft(raw).unwrap();
        assert_eq!(d.questions.len(), 1, "question with no prompt is dropped");
        assert_eq!(d.questions[0].prompt, "Real question?");
        assert_eq!(d.questions[0].slot, Slot::User, "bad slot defaults to user");
        assert_eq!(
            d.questions[0].kind,
            QuestionKind::Content,
            "bad kind defaults to content"
        );
        assert_eq!(d.questions[0].id, "q2", "missing id derived from index");
    }

    #[test]
    fn empty_draft_is_error() {
        assert!(parse_draft(r#"{"user":"","workspace":"","questions":[]}"#).is_err());
    }

    #[test]
    fn invalid_json_is_error() {
        assert!(parse_draft("not json").is_err());
    }
}
