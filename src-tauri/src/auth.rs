use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};

use crate::error::AppResult;

const KEYCHAIN_SERVICE: &str = "chatgpt2api";
const REFRESH_SKEW_MINUTES: i64 = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl AuthTokens {
    pub fn requires_refresh(&self, now: DateTime<Utc>) -> bool {
        self.expires_at <= now + Duration::minutes(REFRESH_SKEW_MINUTES)
    }
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
