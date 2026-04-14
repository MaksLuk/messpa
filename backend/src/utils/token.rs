use sha2::{Sha256, Digest};
use uuid::Uuid;
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;

pub fn create_refresh_token(family_id: Uuid) -> String {
    let mut random = [0u8; 32];
    rand::rng().fill_bytes(&mut random);
    let random_b64 = general_purpose::STANDARD.encode(random);
    let family_b64 = general_purpose::STANDARD.encode(family_id.as_bytes());
    format!("{}.{}", family_b64, random_b64)
}

pub fn parse_refresh_token(token: &str) -> Option<(Uuid, String)> {
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    if parts.len() != 2 {
        return None;
    }
    let family_b64 = parts[0];
    let random_part = parts[1];
    let family_bytes = general_purpose::STANDARD.decode(family_b64).ok()?;
    let family_id = Uuid::from_slice(&family_bytes).ok()?;
    Some((family_id, random_part.to_string()))
}

pub fn hash_token(token_part: &str, pepper: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token_part.as_bytes());
    hasher.update(pepper.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_magic_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_code() -> String {
    format!("{:06}", rand::random::<u32>() % 1_000_000)
}
