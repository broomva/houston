//! OAuth 2.0 install + macOS keychain storage + mutex-guarded refresh.
//!
//! Linear OAuth flow:
//! 1. Engine builds an authorize URL with `client_id`, `redirect_uri`,
//!    `state`, and the requested scopes (`read write app:assignable
//!    app:mentionable webhook:write`).
//! 2. User completes consent in their Linear workspace.
//! 3. Linear redirects to `redirect_uri?code=<code>&state=<state>`.
//! 4. Engine exchanges `code` for access + refresh tokens via
//!    [`crate::LINEAR_OAUTH_TOKEN_URL`].
//! 5. Tokens persist to the macOS keychain (one entry per workspace,
//!    keyed by Linear `org_id`); the on-disk `connection.json` carries
//!    only the opaque keychain ref.
//! 6. Refresh: Linear rotates the refresh token on each use. Refresh
//!    must be single-flight per connection — a `tokio::sync::Mutex`
//!    guards the refresh path. Concurrent refreshers race ⇒ one token
//!    becomes invalid ⇒ user sees auth-required UI.
//!
//! Populated in C2.

use crate::error::LinearError;

/// OAuth scopes Houston requests at install. Listed here so they live
/// next to the spec; the constant is referenced from both authorize-URL
/// construction and the AgentSession capability declaration.
pub const REQUIRED_SCOPES: &[&str] = &[
    "read",
    "write",
    "app:assignable",
    "app:mentionable",
    "webhook:write",
];

/// Build a Linear OAuth authorize URL.
///
/// C2 will return [`url::Url`] with the appropriate query string.
/// Returning [`LinearError::Oauth`] preserves the no-silent-failures
/// contract for invalid input (empty client id, malformed redirect).
pub fn build_authorize_url(
    _client_id: &str,
    _redirect_uri: &str,
    _state: &str,
) -> Result<url::Url, LinearError> {
    Err(LinearError::Oauth(
        "build_authorize_url not yet implemented (C2)".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_scopes_include_agent_session() {
        // The agent_session capability depends on app:assignable + app:mentionable.
        assert!(REQUIRED_SCOPES.contains(&"app:assignable"));
        assert!(REQUIRED_SCOPES.contains(&"app:mentionable"));
    }

    #[test]
    fn required_scopes_include_webhook_write() {
        // Programmatic webhook registration requires webhook:write.
        assert!(REQUIRED_SCOPES.contains(&"webhook:write"));
    }
}
