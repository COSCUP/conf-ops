use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Middleware that adds security headers to all responses.
///
/// Headers added:
/// - `X-Content-Type-Options: nosniff`
/// - `X-Frame-Options: DENY`
/// - `X-XSS-Protection: 0` (modern browsers use CSP instead)
/// - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
/// - `Content-Security-Policy: default-src 'self'; ...`
///
/// # Panics
///
/// Panics if any of the hard-coded header values fail to parse. This should never
/// happen as all values are valid static strings.
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().expect("valid header value"),
    );
    headers.insert(
        "X-Frame-Options",
        "DENY".parse().expect("valid header value"),
    );
    headers.insert("X-XSS-Protection", "0".parse().expect("valid header value"));
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains"
            .parse()
            .expect("valid header value"),
    );
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' wss:; font-src 'self'; frame-ancestors 'none'"
            .parse()
            .expect("valid header value"),
    );

    response
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use super::*;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn security_headers_are_set() {
        let app = Router::new()
            .route("/test", get(ok_handler))
            .layer(middleware::from_fn(security_headers));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );
        assert_eq!(response.headers().get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(response.headers().get("X-XSS-Protection").unwrap(), "0");
        assert!(response
            .headers()
            .get("Strict-Transport-Security")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("max-age=31536000"));
        assert!(response.headers().get("Content-Security-Policy").is_some());
    }
}
