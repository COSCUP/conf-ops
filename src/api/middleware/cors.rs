use axum::http::header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use axum::http::Method;
use tower_http::cors::CorsLayer;

/// Custom header name for API key authentication.
static X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");
/// Custom header name for request tracing.
static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Build CORS layer from a comma-separated origins string.
///
/// Per the deployment doc, allowed methods are:
/// GET, POST, PUT, PATCH, DELETE, OPTIONS
///
/// Allowed headers: Content-Type, Authorization, X-API-Key, X-Request-ID
///
/// Credentials: true, Max age: 86400 (24h)
///
/// # Panics
///
/// Panics if the fallback localhost origin fails to parse. This should never
/// happen as it is a valid static string.
pub fn build_cors_layer(origins: &str) -> CorsLayer {
    let allowed_origins: Vec<HeaderValue> = origins
        .split(',')
        .filter_map(|o| {
            let trimmed = o.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<HeaderValue>().ok()
            }
        })
        .collect();

    let mut layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            CONTENT_TYPE,
            AUTHORIZATION,
            X_API_KEY.clone(),
            X_REQUEST_ID.clone(),
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(86400));

    if allowed_origins.is_empty() {
        // Default: allow localhost for development
        layer = layer.allow_origin(
            "http://localhost:3000"
                .parse::<HeaderValue>()
                .expect("valid header value"),
        );
    } else {
        layer = layer.allow_origin(allowed_origins);
    }

    layer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_cors_layer_with_single_origin() {
        let layer = build_cors_layer("https://confops.dev");
        // Just verify it doesn't panic
        drop(layer);
    }

    #[test]
    fn build_cors_layer_with_multiple_origins() {
        let layer = build_cors_layer("https://confops.dev, https://staging.confops.dev");
        drop(layer);
    }

    #[test]
    fn build_cors_layer_with_empty_string() {
        let layer = build_cors_layer("");
        drop(layer);
    }
}
