use base64::Engine;
use sha2::{Digest, Sha256};

/// Generate a cryptographically random magic link token.
/// Returns `(raw_token_urlsafe_base64, sha256_hash)`.
pub fn generate_magic_link_token() -> (String, Vec<u8>) {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash_magic_link_token(&raw);

    (raw, hash)
}

/// Hash a magic link token using SHA-256.
pub fn hash_magic_link_token(raw: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_unique() {
        let (raw1, _) = generate_magic_link_token();
        let (raw2, _) = generate_magic_link_token();
        assert_ne!(raw1, raw2);
    }

    #[test]
    fn hash_is_deterministic() {
        let hash1 = hash_magic_link_token("test-token");
        let hash2 = hash_magic_link_token("test-token");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn token_is_url_safe() {
        let (raw, _) = generate_magic_link_token();
        assert!(!raw.contains('+'));
        assert!(!raw.contains('/'));
        assert!(!raw.contains('='));
    }
}
