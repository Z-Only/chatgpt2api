use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

pub fn session_id_for_prompt(prompt: &str) -> String {
    let digest = Sha256::digest(prompt.trim().as_bytes());
    format!("sess_{}", URL_SAFE_NO_PAD.encode(&digest[..18]))
}
