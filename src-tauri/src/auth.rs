use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    oauth::parse_jwt_claims,
    upstream::UpstreamClient,
};

const KEYCHAIN_SERVICE: &str = "chatgpt2api";
const REFRESH_SKEW_MINUTES: i64 = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalChatGptCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub account_id: Option<String>,
    pub email: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl AuthTokens {
    pub fn requires_refresh(&self, now: DateTime<Utc>) -> bool {
        self.expires_at <= now + Duration::minutes(REFRESH_SKEW_MINUTES)
    }
}

impl LocalChatGptCredentials {
    pub fn upstream_client(&self, base_url: &str) -> AppResult<UpstreamClient> {
        let session_id = self
            .account_id
            .clone()
            .or_else(|| self.email.clone())
            .unwrap_or_else(|| "local-chatgpt-session".to_string());

        // ponytail: stable enough for local forwarding; add device-id persistence only if upstream rejects it.
        UpstreamClient::new(
            base_url,
            self.access_token.clone(),
            session_id,
            "chatgpt2api",
        )
    }
}

#[derive(Deserialize)]
struct CodexAuthFile {
    tokens: CodexAuthTokens,
}

#[derive(Deserialize)]
struct CodexAuthTokens {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    account_id: Option<String>,
}

pub fn default_codex_auth_path() -> AppResult<PathBuf> {
    if let Some(path) = std::env::var_os("CHATGPT2API_CODEX_AUTH_PATH") {
        return Ok(PathBuf::from(path));
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| AppError::Auth("home directory not found".to_string()))?;
    Ok(PathBuf::from(home).join(".codex").join("auth.json"))
}

pub fn load_local_chatgpt_credentials() -> AppResult<LocalChatGptCredentials> {
    load_local_chatgpt_credentials_from_path(&default_codex_auth_path()?)
}

pub fn load_local_chatgpt_credentials_from_path(path: &Path) -> AppResult<LocalChatGptCredentials> {
    let file: CodexAuthFile = serde_json::from_str(&fs::read_to_string(path)?)
        .map_err(|error| AppError::Auth(format!("invalid Codex auth file: {error}")))?;
    if file.tokens.access_token.trim().is_empty() {
        return Err(AppError::Auth(
            "Codex auth access token is empty".to_string(),
        ));
    }

    let access_claims = parse_jwt_claims(&file.tokens.access_token)?;
    let expires_at = access_claims
        .exp
        .and_then(|exp| Utc.timestamp_opt(exp, 0).single())
        .ok_or_else(|| AppError::Auth("Codex access token is missing expiry".to_string()))?;
    if expires_at <= Utc::now() {
        return Err(AppError::Auth("Codex access token is expired".to_string()));
    }

    let email = file
        .tokens
        .id_token
        .as_deref()
        .and_then(|token| parse_jwt_claims(token).ok())
        .and_then(|claims| claims.email);

    Ok(LocalChatGptCredentials {
        access_token: file.tokens.access_token,
        refresh_token: file.tokens.refresh_token,
        id_token: file.tokens.id_token,
        account_id: file.tokens.account_id,
        email,
        expires_at,
    })
}

#[derive(Clone)]
pub struct TokenStore {
    backend: Arc<TokenStoreBackend>,
}

enum TokenStoreBackend {
    Memory(MemoryTokenStore),
    Keychain(KeychainTokenStore),
}

impl TokenStore {
    pub fn memory_only() -> Self {
        Self {
            backend: Arc::new(TokenStoreBackend::Memory(MemoryTokenStore::default())),
        }
    }

    pub fn keychain_or_memory(account: &str) -> Self {
        KeychainTokenStore::new(account)
            .map(|keychain| Self {
                backend: Arc::new(TokenStoreBackend::Keychain(keychain)),
            })
            .unwrap_or_else(|_| Self::memory_only())
    }

    pub fn store_refresh_token(&self, token: &str) -> AppResult<()> {
        match self.backend.as_ref() {
            TokenStoreBackend::Memory(memory) => memory.store(token),
            TokenStoreBackend::Keychain(keychain) => keychain.store(token),
        }
    }

    pub fn load_refresh_token(&self) -> AppResult<Option<String>> {
        match self.backend.as_ref() {
            TokenStoreBackend::Memory(memory) => memory.load(),
            TokenStoreBackend::Keychain(keychain) => keychain.load(),
        }
    }

    pub fn clear_refresh_token(&self) -> AppResult<()> {
        match self.backend.as_ref() {
            TokenStoreBackend::Memory(memory) => memory.clear(),
            TokenStoreBackend::Keychain(keychain) => keychain.clear(),
        }
    }
}

#[derive(Default)]
struct MemoryTokenStore {
    refresh_token: Mutex<Option<String>>,
}

impl MemoryTokenStore {
    fn store(&self, token: &str) -> AppResult<()> {
        *self
            .refresh_token
            .lock()
            .expect("refresh token lock poisoned") = Some(token.to_string());
        Ok(())
    }

    fn load(&self) -> AppResult<Option<String>> {
        Ok(self
            .refresh_token
            .lock()
            .expect("refresh token lock poisoned")
            .clone())
    }

    fn clear(&self) -> AppResult<()> {
        *self
            .refresh_token
            .lock()
            .expect("refresh token lock poisoned") = None;
        Ok(())
    }
}

struct KeychainTokenStore {
    entry: keyring::Entry,
    memory: MemoryTokenStore,
}

impl KeychainTokenStore {
    fn new(account: &str) -> Result<Self, keyring::Error> {
        Ok(Self {
            entry: keyring::Entry::new(KEYCHAIN_SERVICE, account)?,
            memory: MemoryTokenStore::default(),
        })
    }

    fn store(&self, token: &str) -> AppResult<()> {
        if self.entry.set_password(token).is_err() {
            return self.memory.store(token);
        }

        self.memory.clear()
    }

    fn load(&self) -> AppResult<Option<String>> {
        Ok(self.entry.get_password().ok().or(self.memory.load()?))
    }

    fn clear(&self) -> AppResult<()> {
        let _ = self.entry.delete_credential();
        self.memory.clear()
    }
}
