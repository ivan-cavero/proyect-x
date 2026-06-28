//! Authentication middleware (JWT for local, API keys for remote).

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Simple auth middleware (stub for now).
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // For now, allow all requests
    // In production: validate JWT token from Authorization header
    Ok(next.run(request).await)
}

/// Validate a JWT token (stub).
pub fn validate_token(_token: &str) -> bool {
    // TODO: implement JWT validation
    true
}

/// Generate a JWT token (stub).
pub fn generate_token(_user_id: &str) -> String {
    // TODO: implement JWT generation
    "stub-token".to_string()
}