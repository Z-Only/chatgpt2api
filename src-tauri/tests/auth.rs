use std::fs;
use std::path::PathBuf;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chatgpt2api::auth::{AuthTokens, TokenStore};
use chatgpt2api::oauth::{parse_jwt_claims, pkce_challenge, BrowserCallbackServer};
use chrono::{Duration, TimeZone, Utc};

fn temp_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("chatgpt2api-{name}-{}-{nanos}", std::process::id()))
}

fn jwt_with_payload(payload: &str) -> String {
    format!(
        "header.{}.signature",
        URL_SAFE_NO_PAD.encode(payload.as_bytes())
    )
}

#[test]
fn oauth_pkce_challenge_matches_rfc7636() {
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    assert_eq!(
        pkce_challenge(verifier),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

#[test]
fn oauth_jwt_payload_parses_claims() {
    let token =
        jwt_with_payload(r#"{"sub":"user-123","email":"user@example.com","exp":1700000123}"#);

    let claims = parse_jwt_claims(&token).unwrap();

    assert_eq!(claims.sub.as_deref(), Some("user-123"));
    assert_eq!(claims.email.as_deref(), Some("user@example.com"));
    assert_eq!(claims.exp, Some(1_700_000_123));
}

#[test]
fn auth_refresh_required_inside_five_minute_window() {
    let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let mut tokens = AuthTokens {
        access_token: "access".to_string(),
        refresh_token: Some("refresh".to_string()),
        id_token: None,
        expires_at: now + Duration::minutes(5),
    };

    assert!(tokens.requires_refresh(now));

    tokens.expires_at = now + Duration::minutes(5) + Duration::seconds(1);
    assert!(!tokens.requires_refresh(now));
}

#[tokio::test]
async fn oauth_browser_callback_server_accepts_local_code() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let server = BrowserCallbackServer::new(listener, "expected-state");

    let handle = tokio::spawn(async move { server.wait_for_callback().await.unwrap() });
    let response = reqwest::get(format!(
        "http://{addr}/callback?code=callback-code&state=expected-state"
    ))
    .await
    .unwrap();

    assert!(response.status().is_success());
    let callback = handle.await.unwrap();
    assert_eq!(callback.code, "callback-code");
    assert_eq!(callback.state, "expected-state");
}

#[test]
fn auth_memory_token_store_does_not_write_refresh_tokens_to_disk() {
    let dir = temp_path("memory-token-store");
    fs::create_dir_all(&dir).unwrap();
    let store = TokenStore::memory_only();

    store.store_refresh_token("refresh-secret").unwrap();

    assert_eq!(
        store.load_refresh_token().unwrap().as_deref(),
        Some("refresh-secret")
    );
    assert!(fs::read_dir(&dir).unwrap().next().is_none());
}
