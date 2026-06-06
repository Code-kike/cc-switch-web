//! Gemini (Google) OAuth access-token refresh.
//!
//! GeminiCli providers store a Google OAuth credential blob
//! (`access_token` + `refresh_token` [+ `client_id`/`client_secret`/`expiry_date`])
//! in their provider config. The `ya29.` access token lives only ~1h; without
//! refresh the proxy degrades to sending an expired token as `Authorization:
//! Bearer` and upstream returns 401 (finding **M31**) — unlike Copilot/Codex
//! which auto-refresh.
//!
//! This mirrors `CodexOAuthManager` / `CopilotAuthManager`: an in-memory token
//! cache keyed by `refresh_token`, a per-key refresh lock so concurrent
//! requests refresh only once, and a ~60s-before-expiry refresh window. Unlike
//! those managers there is no persisted multi-account store — the
//! `refresh_token` lives in the provider config — so this is a process-global
//! singleton (`manager()`). It is runtime-agnostic (no Tauri), so it compiles
//! and works in both the desktop and `web-server` builds.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tokio::sync::{Mutex, RwLock};

use super::gemini::OAuthCredentials;

/// Public Gemini CLI OAuth client credentials (from google-gemini/gemini-cli;
/// the same public values used by `services::subscription`). Used only as a
/// fallback when the stored credential blob does not carry its own
/// `client_id` / `client_secret`.
const GEMINI_OAUTH_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const GEMINI_OAUTH_CLIENT_SECRET: &str = "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";

const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// Test-only override for the refresh endpoint (mirrors the Codex OAuth manager).
const TEST_TOKEN_URL_ENV: &str = "CC_SWITCH_TEST_GEMINI_OAUTH_TOKEN_URL";

/// Refresh this many milliseconds before the cached token's expiry.
const REFRESH_BUFFER_MS: i64 = 60_000;
/// Assumed lifetime (seconds) when the token endpoint omits `expires_in`.
const DEFAULT_EXPIRES_IN_SECS: i64 = 3600;
/// Network timeout for the refresh request.
const REFRESH_TIMEOUT_SECS: u64 = 10;

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn token_url() -> String {
    match std::env::var(TEST_TOKEN_URL_ENV) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => DEFAULT_TOKEN_URL.to_string(),
    }
}

#[derive(Clone)]
struct CachedToken {
    token: String,
    /// Expiry as a Unix millisecond timestamp.
    expires_at_ms: i64,
}

impl CachedToken {
    fn is_expiring_soon(&self) -> bool {
        now_ms() >= self.expires_at_ms - REFRESH_BUFFER_MS
    }
}

/// Process-global Gemini OAuth token manager.
#[derive(Default)]
pub struct GeminiOAuthManager {
    /// Cached access tokens keyed by `refresh_token`.
    cache: RwLock<HashMap<String, CachedToken>>,
    /// Per-`refresh_token` locks so concurrent requests refresh only once.
    locks: RwLock<HashMap<String, Arc<Mutex<()>>>>,
}

/// The shared process-global manager.
pub fn manager() -> &'static GeminiOAuthManager {
    static MANAGER: OnceLock<GeminiOAuthManager> = OnceLock::new();
    MANAGER.get_or_init(GeminiOAuthManager::default)
}

impl GeminiOAuthManager {
    /// Return a usable Google OAuth access token for `creds`, refreshing via the
    /// stored `refresh_token` when the current token is missing or expiring.
    ///
    /// Falls back to the stored `access_token` whenever a refresh can't be
    /// performed or fails, so it is never *worse* than sending the raw stored
    /// token. Returns `None` only when there is neither a usable token nor a way
    /// to obtain one.
    pub async fn get_valid_token(&self, creds: &OAuthCredentials) -> Option<String> {
        let refresh_token = match creds.refresh_token.as_deref() {
            Some(token) if !token.trim().is_empty() => token.trim().to_string(),
            // No refresh capability: use the stored access token as-is.
            _ => return creds.non_empty_access_token(),
        };

        // Fast path: a previously-refreshed token that isn't expiring yet.
        if let Some(cached) = self.cache.read().await.get(&refresh_token) {
            if !cached.is_expiring_soon() {
                return Some(cached.token.clone());
            }
        }

        let lock = self.lock_for(&refresh_token).await;
        let _guard = lock.lock().await;

        // Double-check the cache after acquiring the refresh lock.
        if let Some(cached) = self.cache.read().await.get(&refresh_token) {
            if !cached.is_expiring_soon() {
                return Some(cached.token.clone());
            }
        }

        // If the stored access token carries an expiry that is comfortably in
        // the future, trust it and avoid a network round-trip.
        if let (Some(stored), Some(expiry)) = (creds.non_empty_access_token(), creds.expiry_date) {
            if expiry - REFRESH_BUFFER_MS > now_ms() {
                self.cache.write().await.insert(
                    refresh_token,
                    CachedToken {
                        token: stored.clone(),
                        expires_at_ms: expiry,
                    },
                );
                return Some(stored);
            }
        }

        // Otherwise exchange the refresh_token for a fresh access_token.
        match self.refresh(creds, &refresh_token).await {
            Some(fresh) => {
                let token = fresh.token.clone();
                self.cache.write().await.insert(refresh_token, fresh);
                Some(token)
            }
            None => creds.non_empty_access_token(),
        }
    }

    async fn lock_for(&self, refresh_token: &str) -> Arc<Mutex<()>> {
        {
            let locks = self.locks.read().await;
            if let Some(lock) = locks.get(refresh_token) {
                return Arc::clone(lock);
            }
        }
        let mut locks = self.locks.write().await;
        Arc::clone(
            locks
                .entry(refresh_token.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }

    async fn refresh(&self, creds: &OAuthCredentials, refresh_token: &str) -> Option<CachedToken> {
        let client = crate::proxy::http_client::get();

        let client_id = creds
            .client_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(GEMINI_OAUTH_CLIENT_ID);
        let client_secret = creds
            .client_secret
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(GEMINI_OAUTH_CLIENT_SECRET);

        let response = client
            .post(token_url())
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .timeout(Duration::from_secs(REFRESH_TIMEOUT_SECS))
            .send()
            .await
            .ok()?;

        let status = response.status();
        if !status.is_success() {
            log::warn!("[Gemini OAuth] refresh failed: HTTP {status}");
            return None;
        }

        let body: serde_json::Value = response.json().await.ok()?;
        let token = body.get("access_token")?.as_str()?.to_string();
        if token.is_empty() {
            return None;
        }
        let expires_in = body
            .get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(DEFAULT_EXPIRES_IN_SECS);

        Some(CachedToken {
            token,
            expires_at_ms: now_ms() + expires_in * 1000,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds(
        access_token: &str,
        refresh_token: Option<&str>,
        expiry_date: Option<i64>,
    ) -> OAuthCredentials {
        OAuthCredentials {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(str::to_string),
            client_id: None,
            client_secret: None,
            expiry_date,
        }
    }

    #[tokio::test]
    async fn no_refresh_token_uses_stored_access_token() {
        let mgr = GeminiOAuthManager::default();
        let got = mgr.get_valid_token(&creds("ya29.stored", None, None)).await;
        assert_eq!(got.as_deref(), Some("ya29.stored"));
    }

    #[tokio::test]
    async fn no_refresh_token_and_empty_access_token_returns_none() {
        let mgr = GeminiOAuthManager::default();
        let got = mgr.get_valid_token(&creds("", None, None)).await;
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn stored_token_with_future_expiry_skips_network() {
        let mgr = GeminiOAuthManager::default();
        let future = now_ms() + 3_600_000; // 1h out
                                           // No token endpoint is configured; if this tried to refresh it would
                                           // hit the network. It must return the stored token directly instead.
        let got = mgr
            .get_valid_token(&creds("ya29.fresh", Some("1//rt"), Some(future)))
            .await;
        assert_eq!(got.as_deref(), Some("ya29.fresh"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn expired_token_refresh_failure_falls_back_to_stored() {
        // Point the refresh endpoint at a closed port so the request fails fast
        // (connection refused) without reaching real Google servers.
        std::env::set_var(TEST_TOKEN_URL_ENV, "http://127.0.0.1:1/token");

        let mgr = GeminiOAuthManager::default();
        let past = now_ms() - 10_000; // already expired
        let got = mgr
            .get_valid_token(&creds(
                "ya29.stale",
                Some("1//rt-expired-fallback"),
                Some(past),
            ))
            .await;
        // Refresh failed → falls back to the stored (stale) token rather than
        // dropping auth entirely.
        assert_eq!(got.as_deref(), Some("ya29.stale"));

        std::env::remove_var(TEST_TOKEN_URL_ENV);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn refresh_exchanges_token_and_caches_it() {
        // Spin a one-shot mock token endpoint.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            use std::io::{Read, Write};
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let body = r#"{"access_token":"ya29.NEW","expires_in":3600}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.flush();
            }
        });

        std::env::set_var(TEST_TOKEN_URL_ENV, format!("http://{addr}/token"));

        let mgr = GeminiOAuthManager::default();
        let past = now_ms() - 10_000;
        let rt = "1//rt-exchange";
        let got = mgr
            .get_valid_token(&creds("ya29.old", Some(rt), Some(past)))
            .await;
        assert_eq!(
            got.as_deref(),
            Some("ya29.NEW"),
            "should use refreshed token"
        );

        // Cached: a second call returns the refreshed token without needing the
        // (now-closed) endpoint.
        let cached = mgr
            .get_valid_token(&creds("ya29.old", Some(rt), Some(past)))
            .await;
        assert_eq!(cached.as_deref(), Some("ya29.NEW"));

        std::env::remove_var(TEST_TOKEN_URL_ENV);
        let _ = handle.join();
    }
}
