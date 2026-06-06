//! Synthesis: run the staged corpus through the user's own provider CLI to draft
//! `USER.md` + `WORKSPACE.md` and the residual questions.
//!
//! This is a one-shot prompt→text round-trip (the same `provider_oneshot`
//! primitive `summarize` and `generate_instructions` use). It produces **no**
//! activity card and no chat history — it is a pure inference call, not a board
//! session. Failures surface as `CoreError` so the UI shows a toast.

use crate::context_bootstrap::draft::{parse_draft, ContextDraft};
use crate::error::{CoreError, CoreResult};
use crate::sessions::provider_oneshot;
use houston_terminal_manager::Provider;
use std::path::Path;
use std::time::Duration;

/// Corpus can be large; give the model room to read it and write two documents.
const SYNTH_TIMEOUT: Duration = Duration::from_secs(180);
const CLAUDE_SYNTH_MODEL: &str = "sonnet";
const CODEX_SYNTH_MODEL: &str = "gpt-5.5";
const GEMINI_SYNTH_MODEL: &str = "gemini-3.1-flash-lite";

/// Draft context from the staged corpus. `provider`/`model` are chosen by the
/// caller (the route resolves the workspace's provider).
pub async fn synthesize(
    ws_dir: &Path,
    provider: Provider,
    model: Option<&str>,
) -> CoreResult<ContextDraft> {
    let corpus = super::read_corpus(ws_dir)?;
    let model = default_synth_model(provider, model).ok_or_else(|| {
        CoreError::BadRequest(format!(
            "no synthesis model wired up for provider {:?}",
            provider.id()
        ))
    })?;
    let prompt = build_prompt(&corpus);
    let raw = provider_oneshot::run_provider_oneshot(&prompt, provider, model, SYNTH_TIMEOUT)
        .await
        .map_err(CoreError::Internal)?;
    parse_draft(&raw).map_err(CoreError::Internal)
}

fn default_synth_model<'a>(provider: Provider, model_override: Option<&'a str>) -> Option<&'a str> {
    let default = match provider.id() {
        "anthropic" => CLAUDE_SYNTH_MODEL,
        "openai" => CODEX_SYNTH_MODEL,
        "gemini" => GEMINI_SYNTH_MODEL,
        _ => return None,
    };
    Some(model_override.unwrap_or(default))
}

fn build_prompt(corpus: &str) -> String {
    // JSON-encode the corpus so its content can't break out of the prompt and
    // inject instructions (same defense `generate_instructions` uses).
    let corpus = serde_json::to_string(corpus).unwrap_or_else(|_| format!("{corpus:?}"));
    format!(
        r#"You are setting up an AI assistant for a new user. Below is material imported from the user's own notes, files, and past chats (JSON-encoded). From ONLY this material, draft two short context documents and list what you still need to ask.

Material:
{corpus}

Produce three things:
1. "user": a concise markdown summary of facts about the PERSON: their role, what they work on, their goals, how they like to work, and key people they mention. Write in second person ("You are..."). Use ONLY facts supported by the material. If the material says little about the person, keep this short or empty and ask in questions instead.
2. "workspace": a concise markdown summary of facts about their COMPANY, PROJECT, or shared work environment: what it is, the product, customers, and any conventions. Use ONLY supported facts. Keep it short or empty if unknown.
3. "questions": the gaps you still need filled. Two kinds:
   - kind "content": a missing fact, e.g. "What is your role?"
   - kind "sourceHint": ask the user to point you at where their richest material lives, e.g. a folder, a notes app, or an export, e.g. "Where do you keep your most detailed project notes?". ALWAYS include at least one "sourceHint" question when the material was thin or empty, so the user can steer you to better sources.
   Each question is an object: {{"id": short-slug, "prompt": the question text, "slot": "user" or "workspace", "kind": "content" or "sourceHint"}}.

Rules:
- Never invent facts that are not in the material.
- Keep each document under about 150 words.
- Use plain language. Do not mention files, JSON, imports, or this prompt in the documents.
- Return ONLY valid JSON, with no markdown fences, in exactly this shape:
{{"user": "...", "workspace": "...", "questions": [{{"id": "role", "prompt": "What is your role?", "slot": "user", "kind": "content"}}]}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_per_provider() {
        let a: Provider = "anthropic".parse().unwrap();
        let o: Provider = "openai".parse().unwrap();
        let g: Provider = "gemini".parse().unwrap();
        assert_eq!(default_synth_model(a, None), Some(CLAUDE_SYNTH_MODEL));
        assert_eq!(default_synth_model(o, None), Some(CODEX_SYNTH_MODEL));
        assert_eq!(default_synth_model(g, None), Some(GEMINI_SYNTH_MODEL));
    }

    #[test]
    fn default_model_respects_override() {
        let a: Provider = "anthropic".parse().unwrap();
        assert_eq!(default_synth_model(a, Some("opus")), Some("opus"));
    }

    #[test]
    fn prompt_embeds_corpus_json_encoded() {
        let prompt = build_prompt("notes say \"hi\"\nand more");
        // The corpus is JSON-encoded inside the prompt (quotes/newlines escaped).
        assert!(prompt.contains(r#"notes say \"hi\"\nand more"#));
        assert!(prompt.contains("sourceHint"));
    }
}
