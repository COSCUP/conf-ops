use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use uuid::Uuid;
use webauthn_rs::prelude::*;
use webauthn_rs::Webauthn;

use crate::config::AppConfig;

/// In-memory challenge store with TTL (5 minutes).
pub struct ChallengeStore<T: Send + Sync> {
    inner: DashMap<Uuid, (T, std::time::Instant)>,
    ttl: Duration,
}

impl<T: Send + Sync> ChallengeStore<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: DashMap::new(),
            ttl,
        }
    }

    pub fn insert(&self, key: Uuid, value: T) {
        self.inner.insert(key, (value, std::time::Instant::now()));
    }

    pub fn remove(&self, key: &Uuid) -> Option<T> {
        let (_, (value, created_at)) = self.inner.remove(key)?;
        if created_at.elapsed() > self.ttl {
            return None;
        }
        Some(value)
    }
}

/// Build a `Webauthn` instance from app config.
///
/// # Errors
///
/// Returns an error string if the config is invalid.
pub fn build_webauthn(config: &AppConfig) -> Result<Arc<Webauthn>, String> {
    let rp_id = config.webauthn_rp_id.clone();
    let rp_origin = url::Url::parse(&config.webauthn_rp_origin)
        .map_err(|e| format!("Invalid RP origin: {e}"))?;

    let builder = WebauthnBuilder::new(&rp_id, &rp_origin)
        .map_err(|e| format!("WebauthnBuilder error: {e}"))?
        .rp_name(&config.webauthn_rp_name);

    Ok(Arc::new(
        builder
            .build()
            .map_err(|e| format!("Webauthn build error: {e}"))?,
    ))
}
