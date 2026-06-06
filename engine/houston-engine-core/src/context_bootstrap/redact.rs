//! Obvious-secret redaction. Runs on every document before it joins the corpus
//! so credentials never reach the synthesis prompt or the staged file.
//! Deliberately conservative — better to leave a borderline string than to
//! mangle prose. The synthesis step needs context, not credentials.

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
        (Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}").unwrap(), "[REDACTED]"),
        (Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap(), "[REDACTED]"),
        (Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), "[REDACTED]"),
        (Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap(), "[REDACTED]"),
        (Regex::new(r"xox[baprs]-[A-Za-z0-9-]{10,}").unwrap(), "[REDACTED]"),
        // key/value secrets: keep the label, redact the value.
        (
            Regex::new(r#"(?i)(api[_-]?key|secret|token|password|passwd|bearer)(\s*[:=]\s*)([^\s'";,]+)"#)
                .unwrap(),
            "$1$2[REDACTED]",
        ),
    ]
});

/// Replace obvious secrets in `input`.
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
}
