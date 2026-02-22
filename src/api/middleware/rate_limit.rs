use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;

use crate::api::error::ProblemDetails;

/// Rate limiter state shared across requests.
#[derive(Clone)]
pub struct RateLimiter {
    /// IP-based rate limit entries: IP -> (count, window start)
    entries: Arc<DashMap<RateLimitKey, RateLimitEntry>>,
    /// Maximum requests per window.
    max_requests: u64,
    /// Window duration in seconds.
    window_secs: u64,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum RateLimitKey {
    Ip(IpAddr),
    IpPath(IpAddr, String),
}

#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u64,
    window_start: std::time::Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given max requests per window.
    pub fn new(max_requests: u64, window_secs: u64) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_requests,
            window_secs,
        }
    }

    /// Check if a request should be rate-limited. Returns `true` if allowed.
    fn check(&self, key: RateLimitKey) -> bool {
        let now = std::time::Instant::now();
        let window_duration = std::time::Duration::from_secs(self.window_secs);

        let mut entry = self.entries.entry(key).or_insert_with(|| RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(entry.window_start) >= window_duration {
            entry.count = 0;
            entry.window_start = now;
        }

        entry.count += 1;
        entry.count <= self.max_requests
    }

    /// Periodically clean up expired entries.
    pub fn start_cleanup(self) {
        let entries = Arc::clone(&self.entries);
        let window_secs = self.window_secs;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(window_secs));
            loop {
                interval.tick().await;
                let now = std::time::Instant::now();
                let window_duration = std::time::Duration::from_secs(window_secs);
                entries.retain(|_, entry| now.duration_since(entry.window_start) < window_duration);
            }
        });
    }
}

/// Global rate limiter: 100 requests per second per IP.
pub static GLOBAL_RATE_LIMITER: std::sync::LazyLock<RateLimiter> =
    std::sync::LazyLock::new(|| RateLimiter::new(100, 1));

/// Auth endpoint rate limiter: 10 requests per minute per IP.
pub static AUTH_RATE_LIMITER: std::sync::LazyLock<RateLimiter> =
    std::sync::LazyLock::new(|| RateLimiter::new(10, 60));

/// AI suggestion endpoint rate limiter: 5 requests per minute per IP+path.
pub static AI_RATE_LIMITER: std::sync::LazyLock<RateLimiter> =
    std::sync::LazyLock::new(|| RateLimiter::new(5, 60));

/// Extract client IP from the request.
fn extract_ip(req: &Request) -> IpAddr {
    // Try X-Forwarded-For header first (for reverse proxy setups)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            if let Some(first_ip) = value.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Fall back to ConnectInfo
    req.extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), |ci| ci.0.ip())
}

fn rate_limit_response() -> Response {
    let problem = ProblemDetails::new(StatusCode::TOO_MANY_REQUESTS, "Too Many Requests")
        .with_detail("Rate limit exceeded. Please retry later.".to_string());
    let mut response = problem.into_response();
    response
        .headers_mut()
        .insert("Retry-After", "60".parse().expect("valid header value"));
    response
}

/// Global rate limit middleware.
pub async fn global_rate_limit(req: Request, next: Next) -> Response {
    let ip = extract_ip(&req);

    if !GLOBAL_RATE_LIMITER.check(RateLimitKey::Ip(ip)) {
        return rate_limit_response();
    }

    next.run(req).await
}

/// Auth endpoint rate limit middleware.
pub async fn auth_rate_limit(req: Request, next: Next) -> Response {
    let ip = extract_ip(&req);

    if !AUTH_RATE_LIMITER.check(RateLimitKey::Ip(ip)) {
        return rate_limit_response();
    }

    next.run(req).await
}

/// AI suggestion endpoint rate limit middleware.
pub async fn ai_rate_limit(req: Request, next: Next) -> Response {
    let ip = extract_ip(&req);
    let path = req.uri().path().to_string();

    if !AI_RATE_LIMITER.check(RateLimitKey::IpPath(ip, path)) {
        return rate_limit_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3, 60);
        let key = RateLimitKey::Ip(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));

        assert!(limiter.check(key.clone()));
        assert!(limiter.check(key.clone()));
        assert!(limiter.check(key.clone()));
        assert!(!limiter.check(key));
    }

    #[test]
    fn rate_limiter_different_ips_independent() {
        let limiter = RateLimiter::new(1, 60);
        let key1 = RateLimitKey::Ip(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
        let key2 = RateLimitKey::Ip(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)));

        assert!(limiter.check(key1.clone()));
        assert!(!limiter.check(key1));
        assert!(limiter.check(key2));
    }
}
