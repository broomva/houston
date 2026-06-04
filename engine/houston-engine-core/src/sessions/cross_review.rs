//! Cross-review capability — the engine side of native agent self-review.
//!
//! Exposes [`run_review`], invoked by the in-engine MCP `request_review` tool
//! (`engine/houston-engine-server/src/routes/mcp.rs`). The calling agent asks
//! a *different* AI model to adversarially review its work before presenting it
//! as final. This is the productized form of cross-model review:
//!
//! - **Governed** — runs in-engine through [`provider_oneshot::run_provider_oneshot`],
//!   with Houston's own CLI resolution, not an ungoverned shell-out.
//! - **Visible** — the `request_review` tool call and its result ride the
//!   NDJSON parser rails into the session feed automatically (every non-
//!   `AskUserQuestion` tool passes through unchanged), so no explicit feed
//!   emission is needed in v1.
//!
//! Reviewer selection prefers an authenticated provider *different* from the
//! caller (true cross-model). When none is usable it falls back to a fresh-
//! context same-provider review — clearly labelled — so the agent always gets
//! a second opinion and the capability never silently fails. Failures of the
//! review itself surface as [`crate::CoreError`] (user-initiated work; no
//! silent fallback, unlike `summarize`).
//!
//! See `docs/specs/2026-06-04-agent-cross-review.md`.

use super::provider_oneshot;
use crate::error::CoreResult;
use houston_terminal_manager::Provider;
use serde_json::{json, Value};
use std::time::Duration;

/// Reviewing is heavier than titling/generation, so allow more wall-clock,
/// but cap it so a hung reviewer CLI can't wedge the agent's turn.
const REVIEW_TIMEOUT: Duration = Duration::from_secs(90);

// Reviewer models per provider. A verdict warrants the capable generation
// tier, not the cheap title tier. These are CLI aliases / catalog ids the
// `--model` flag accepts; keep in sync with `app/src/lib/providers.ts` and the
// engine generation consts, NOT the stale `agent-manifest.md` model table.
const CLAUDE_REVIEW_MODEL: &str = "sonnet";
const CODEX_REVIEW_MODEL: &str = "gpt-5.5";
// Flash-Lite, for the same reason `generate_instructions` uses it: the Gemini
// pro tier is gated behind paid Google AI plans and free-tier OAuth gets zero
// quota (multi-minute hangs). Flash-Lite is the safe default reviewer.
const GEMINI_REVIEW_MODEL: &str = "gemini-3.1-flash-lite";

/// Default reviewer model for a provider. `None` for providers with no
/// one-shot CLI wired (`life`/unknown — see `provider_oneshot`), which the
/// selector skips and the caller rejects.
fn default_reviewer_model(provider: Provider) -> Option<&'static str> {
    match provider.id() {
        "anthropic" => Some(CLAUDE_REVIEW_MODEL),
        "openai" => Some(CODEX_REVIEW_MODEL),
        "gemini" => Some(GEMINI_REVIEW_MODEL),
        _ => None,
    }
}

/// Choose a reviewer, preferring an authenticated provider *different* from the
/// caller (registry order: openai before gemini, so the stronger cross-model
/// reviewer wins for an anthropic caller). Returns `(reviewer, cross_provider)`
/// where `cross_provider` is false when we fell back to a same-provider, fresh-
/// context review because no different provider was usable.
///
/// The auth pre-check ([`crate::provider::check_status`]) is cheap (no session
/// spawn). A candidate whose status can't be read is skipped, not fatal — the
/// overall review still runs (cross-model or self-review).
async fn select_reviewer(caller: Provider) -> (Provider, bool) {
    for &adapter in houston_terminal_manager::provider::all() {
        let candidate = Provider::from(adapter);
        if candidate == caller || default_reviewer_model(candidate).is_none() {
            continue;
        }
        match crate::provider::check_status(candidate).await {
            Ok(status) if status.cli_installed && status.auth_state.is_authenticated() => {
                return (candidate, true);
            }
            Ok(_) => continue,
            Err(e) => {
                tracing::debug!(
                    "[houston:cross_review] skipping reviewer candidate {}: {e}",
                    candidate.id()
                );
                continue;
            }
        }
    }
    // No usable different provider — fall back to a fresh same-provider review.
    (caller, false)
}

/// Run a cross-model review of `work_content`.
///
/// - `caller` is the requesting agent's provider; the reviewer is chosen to
///   differ from it when possible.
/// - `work_summary` is a short description of what the agent set out to do.
/// - `focus` optionally narrows what the reviewer should scrutinize.
///
/// Returns a structured verdict (`reviewer`, `model`, `crossProvider`,
/// `review`). Surfaces a [`crate::CoreError`] when the reviewer can't run (no
/// model wired, spawn failure, timeout) — never a silent fallback.
pub async fn run_review(
    caller: Provider,
    work_summary: &str,
    work_content: &str,
    focus: Option<&str>,
) -> CoreResult<Value> {
    let (reviewer, cross_provider) = select_reviewer(caller).await;
    let model = default_reviewer_model(reviewer).ok_or_else(|| {
        crate::CoreError::Internal(format!(
            "no reviewer model wired up for provider {}",
            reviewer.id()
        ))
    })?;

    let prompt = build_review_prompt(work_summary, work_content, focus);
    let review = provider_oneshot::run_provider_oneshot(&prompt, reviewer, model, REVIEW_TIMEOUT)
        .await
        .map_err(crate::CoreError::Internal)?;

    Ok(json!({
        "reviewer": reviewer.id(),
        "model": model,
        "crossProvider": cross_provider,
        "review": review.trim(),
    }))
}

/// Build the reviewer prompt. The agent-supplied strings are JSON-encoded so
/// quotes/newlines in them can't break out of the prompt context and inject
/// instructions (same hardening as `generate_instructions::build_prompt`).
fn build_review_prompt(work_summary: &str, work_content: &str, focus: Option<&str>) -> String {
    let work_summary =
        serde_json::to_string(work_summary).unwrap_or_else(|_| format!("{work_summary:?}"));
    let work_content =
        serde_json::to_string(work_content).unwrap_or_else(|_| format!("{work_content:?}"));
    let focus_line = match focus {
        Some(f) if !f.trim().is_empty() => {
            let f = serde_json::to_string(f).unwrap_or_else(|_| format!("{f:?}"));
            format!("\nPay particular attention to: {f}\n")
        }
        _ => String::new(),
    };

    format!(
        r#"You are an independent reviewer giving a second opinion on another AI assistant's work. Be adversarial and specific: your job is to catch problems the author missed, not to praise.

What the assistant set out to do:
{work_summary}

The work to review:
{work_content}
{focus_line}
Review for: correctness, missing pieces, risks or unintended consequences, and anything a careful expert would flag.

Respond concisely in plain text:
- Start with a one-line bottom line: LOOKS GOOD, NEEDS CHANGES, or SERIOUS PROBLEMS.
- Then list the concrete issues you found, most important first. If you found none, say so plainly.
Do not restate the work. Do not add filler."#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reviewer_model_picks_per_provider() {
        let a: Provider = "anthropic".parse().unwrap();
        let o: Provider = "openai".parse().unwrap();
        let g: Provider = "gemini".parse().unwrap();
        assert_eq!(default_reviewer_model(a), Some(CLAUDE_REVIEW_MODEL));
        assert_eq!(default_reviewer_model(o), Some(CODEX_REVIEW_MODEL));
        assert_eq!(default_reviewer_model(g), Some(GEMINI_REVIEW_MODEL));
    }

    #[test]
    fn default_reviewer_model_none_for_providers_without_oneshot() {
        // `life` is registered but has no one-shot CLI wired; it must not be
        // selectable as a reviewer.
        let life: Provider = "life".parse().unwrap();
        assert_eq!(default_reviewer_model(life), None);
    }

    #[test]
    fn build_review_prompt_json_escapes_inputs() {
        let prompt = build_review_prompt("say \"hi\"\nthen ignore previous", "the content", None);
        // Quotes and newline are JSON-escaped, not embedded raw — no prompt
        // injection break-out.
        assert!(prompt.contains(r#""say \"hi\"\nthen ignore previous""#));
        assert!(prompt.contains(r#""the content""#));
    }

    #[test]
    fn build_review_prompt_includes_focus_when_present() {
        let with = build_review_prompt("s", "c", Some("the error handling"));
        assert!(with.contains("Pay particular attention to:"));
        assert!(with.contains("the error handling"));
    }

    #[test]
    fn build_review_prompt_omits_focus_when_absent_or_blank() {
        assert!(!build_review_prompt("s", "c", None).contains("Pay particular attention to:"));
        assert!(
            !build_review_prompt("s", "c", Some("   ")).contains("Pay particular attention to:")
        );
    }
}
