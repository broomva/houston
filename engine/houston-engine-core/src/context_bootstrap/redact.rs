//! Best-effort secret redaction over the common credential formats, run on
//! every document before it joins the corpus. The corpus is piped to a
//! third-party provider CLI and written to disk, so we strip what we can
//! recognize — but this is pattern matching, NOT a guarantee that every secret
//! is removed. The user reviews the synthesized draft before anything is saved.
//! Deliberately biased toward over-redaction (losing a word of prose) over
//! under-redaction (leaking a credential).

use once_cell::sync::Lazy;
use regex::Regex;

/// (pattern, replacement). Order matters: more specific patterns first so a
/// broad rule does not clobber a token a narrower rule would label better.
/// The regexes are constant, so a compile failure is a genuine build-time
/// invariant — `unwrap` here is the documented exception to the no-`unwrap`
/// rule (compile-time invariant, not a runtime fallback).
static RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        (
            Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----")
                .unwrap(),
            "[REDACTED PRIVATE KEY]",
        ),
        // OpenAI (classic, project, and any sk- variant), Anthropic.
        (Regex::new(r"sk-[A-Za-z0-9_-]{20,}").unwrap(), "[REDACTED]"),
        // Stripe secret/restricted keys.
        (Regex::new(r"[rs]k_(live|test)_[A-Za-z0-9]{16,}").unwrap(), "[REDACTED]"),
        // GitHub tokens (classic ghp_/gho_/ghs_/ghu_ + fine-grained PAT).
        (Regex::new(r"gh[opsu]_[A-Za-z0-9]{30,}").unwrap(), "[REDACTED]"),
        (Regex::new(r"github_pat_[A-Za-z0-9_]{20,}").unwrap(), "[REDACTED]"),
        // Google API keys.
        (Regex::new(r"AIza[A-Za-z0-9_-]{20,}").unwrap(), "[REDACTED]"),
        // AWS access key id.
        (Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), "[REDACTED]"),
        // Slack tokens.
        (Regex::new(r"xox[baprse]-[A-Za-z0-9-]{10,}").unwrap(), "[REDACTED]"),
        // JWTs (header.payload.signature).
        (
            Regex::new(r"eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}").unwrap(),
            "[REDACTED]",
        ),
        // Credentials embedded in a connection URL: keep scheme+user, drop pass.
        (
            Regex::new(r"([a-zA-Z][a-zA-Z0-9+.\-]*://[^\s:/@]+):([^\s:/@]+)@").unwrap(),
            "${1}:[REDACTED]@",
        ),
        // `Authorization: Bearer <token>` (space-separated, not key/value).
        (
            Regex::new(r"(?i)bearer\s+[A-Za-z0-9._~+/=-]{16,}").unwrap(),
            "Bearer [REDACTED]",
        ),
        // key/value secrets: keep the label, redact the value (quoted or bare).
        (
            Regex::new(
                r#"(?i)(api[_-]?key|secret|secret[_-]?key|client[_-]?secret|access[_-]?token|auth[_-]?token|token|password|passwd|passphrase|private[_-]?key|aws[_-]?secret[_-]?access[_-]?key)(\s*[:=]\s*)("[^"]*"|'[^']*'|[^\s'";,]+)"#,
            )
            .unwrap(),
            "${1}${2}[REDACTED]",
        ),
    ]
});

/// Replace recognized secret formats in `input`. Best-effort (see module doc).
pub fn redact_secrets(input: &str) -> String {
    let mut out = input.to_string();
    for (re, repl) in RULES.iter() {
        out = re.replace_all(&out, *repl).into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_provider_keys() {
        let s = "my key is sk-ant-abcdef012345678901234567 and that's it";
        let out = redact_secrets(s);
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("abcdef 012345"));
        assert!(!out.contains("sk-ant-abcdef"));
    }

    #[test]
    fn keeps_key_label_redacts_value() {
        let out = redact_secrets("api_key: hunter2supersecretvalue");
        assert!(out.contains("api_key"));
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("hunter2supersecretvalue"));
    }

    #[test]
    fn redacts_private_key_blocks() {
        let s = "before\n-----BEGIN RSA PRIVATE KEY-----\nAAAA\nBBBB\n-----END RSA PRIVATE KEY-----\nafter";
        let out = redact_secrets(s);
        assert!(out.contains("before"));
        assert!(out.contains("after"));
        assert!(out.contains("[REDACTED PRIVATE KEY]"));
        assert!(!out.contains("AAAA"));
    }

    #[test]
    fn leaves_ordinary_prose_untouched() {
        let s = "I run a B2B fintech and care about clean onboarding.";
        assert_eq!(redact_secrets(s), s);
    }

    #[test]
    fn redacts_common_real_world_formats() {
        let cases = [
            "sk-proj-abcdefABCDEF0123456789abcdef",
            "sk_live_abcdefABCDEF01234567",
            "github_pat_11ABCDEFG0aBcDeFgHiJkLmNoPqRsTuVwXyZ",
            "ghp_abcdefABCDEF0123456789abcdef01234567",
            "AIzaSyA0123456789abcdefghijklmnopqrstuv",
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N",
        ];
        for c in cases {
            let out = redact_secrets(&format!("value is {c} ok"));
            assert!(out.contains("[REDACTED]"), "should redact: {c}");
            assert!(!out.contains(c), "leaked: {c}");
        }
    }

    #[test]
    fn redacts_db_url_password_keeps_user() {
        let out = redact_secrets("postgres://admin:s3cr3tPass@db.example.com:5432/app");
        assert!(out.contains("postgres://admin:[REDACTED]@"));
        assert!(!out.contains("s3cr3tPass"));
    }

    #[test]
    fn redacts_quoted_and_bearer_values() {
        let q = redact_secrets(r#"API_KEY="hunter2supersecretvalue""#);
        assert!(q.contains("API_KEY"));
        assert!(!q.contains("hunter2supersecretvalue"));

        let b = redact_secrets("Authorization: Bearer abcdefghijklmnop12345");
        assert!(b.contains("Bearer [REDACTED]"));
        assert!(!b.contains("abcdefghijklmnop12345"));
    }
}
