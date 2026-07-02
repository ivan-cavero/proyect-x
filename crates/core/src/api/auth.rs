//! Authentication middleware — JWT tokens for local API access.
//!
//! Uses HMAC-SHA256 for signing (simple, no external key server needed).
//! Tokens expire after 24 hours by default.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT secret key (generated once, stored in .forge/jwt.secret).
/// In production, this should be at least 32 bytes.
/// JWT token claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: String,
    /// Issued at (Unix timestamp).
    pub iat: u64,
    /// Expiration (Unix timestamp).
    pub exp: u64,
    /// User role.
    pub role: String,
}

/// Authentication state shared across handlers.
#[derive(Clone)]
pub struct AuthState {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    /// Token expiry in seconds (default: 24 hours).
    pub token_expiry_secs: u64,
}

impl AuthState {
    /// Create auth state with a secret key.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            token_expiry_secs: 86400, // 24 hours
        }
    }

    /// Create with custom expiry.
    pub fn with_expiry(secret: &[u8], expiry_secs: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            token_expiry_secs: expiry_secs,
        }
    }

    /// Load secret from file, or generate and save a new one.
    pub fn from_file_or_create(secret_path: &std::path::Path) -> Self {
        let secret = if secret_path.exists() {
            match std::fs::read(secret_path) {
                Ok(bytes) if bytes.len() >= 32 => bytes,
                _ => {
                    tracing::warn!("JWT secret too short or unreadable, regenerating");
                    Self::generate_and_save(secret_path)
                }
            }
        } else {
            Self::generate_and_save(secret_path)
        };

        Self::new(&secret)
    }

    /// Generate a random secret and save it to a file.
    fn generate_and_save(secret_path: &std::path::Path) -> Vec<u8> {
        use rand::RngCore;
        let mut secret = vec![0u8; 64];
        rand::rng().fill_bytes(&mut secret);

        if let Some(parent) = secret_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(secret_path, &secret) {
            tracing::error!("Failed to save JWT secret: {}", e);
        } else {
            tracing::info!("Generated new JWT secret at {}", secret_path.display());
        }

        secret
    }

    /// Generate a JWT token for a user.
    pub fn generate_token(
        &self,
        user_id: &str,
        role: &str,
    ) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as u64;

        let claims = Claims {
            sub: user_id.to_string(),
            iat: now,
            exp: now + self.token_expiry_secs,
            role: role.to_string(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenCreation(e.to_string()))
    }

    /// Validate a JWT token and return its claims.
    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => AuthError::InvalidSignature,
            _ => AuthError::InvalidToken(e.to_string()),
        })?;

        Ok(token_data.claims)
    }
}

/// Authentication errors.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token creation failed: {0}")]
    TokenCreation(String),

    #[error("No token provided")]
    NoToken,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl std::fmt::Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "user={}, role={}", self.sub, self.role)
    }
}

// ─── Middleware ────────────────────────────────────────────────

/// Extract the Bearer token from the Authorization header.
fn extract_bearer_token(request: &Request) -> Option<String> {
    let auth_header = request.headers().get(header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;
    auth_str.strip_prefix("Bearer ").map(|s| s.to_string())
}

/// Authentication middleware — validates JWT tokens on protected routes.
///
/// Routes that start with `/api/health` or `/ws/` are exempt from auth.
pub async fn auth_middleware(
    State(auth): State<Arc<AuthState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    // Exempt health check and WebSocket upgrade paths
    if path == "/api/health" || path.starts_with("/ws/") {
        return Ok(next.run(request).await);
    }

    // Extract token
    let token = match extract_bearer_token(&request) {
        Some(t) => t,
        None => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate
    match auth.validate_token(&token) {
        Ok(_claims) => Ok(next.run(request).await),
        Err(AuthError::TokenExpired) => Err(StatusCode::UNAUTHORIZED),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Generate a first-run admin token (for local-only usage).
pub fn generate_first_run_token(auth: &AuthState) -> Result<String, AuthError> {
    auth.generate_token("admin", "admin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_roundtrip() {
        let auth = AuthState::new(b"test-secret-key-at-least-32-bytes-long!");
        let token = auth.generate_token("user1", "developer").unwrap();
        let claims = auth.validate_token(&token).unwrap();

        assert_eq!(claims.sub, "user1");
        assert_eq!(claims.role, "developer");
    }

    #[test]
    fn test_token_expiry() {
        // Test that tokens have exp claim set correctly
        let auth = AuthState::with_expiry(b"test-secret-key-at-least-32-bytes-long!", 3600);
        let token = auth.generate_token("user1", "admin").unwrap();
        let claims = auth.validate_token(&token).unwrap();

        // exp should be ~3600 seconds in the future
        let now = chrono::Utc::now().timestamp() as u64;
        assert!(claims.exp > now, "exp should be in the future");
        assert!(claims.exp <= now + 3601, "exp should be ~3600s from now");
    }

    #[test]
    fn test_invalid_signature() {
        let auth1 = AuthState::new(b"secret-key-number-one-32-bytes-long!!!!");
        let auth2 = AuthState::new(b"secret-key-number-two-32-bytes-long!!!!");

        let token = auth1.generate_token("user1", "admin").unwrap();
        let result = auth2.validate_token(&token);
        assert!(result.is_err(), "Should fail with wrong key: {:?}", result);
    }

    #[test]
    fn test_first_run_token() {
        let auth = AuthState::new(b"test-secret-for-first-run-token-32bytes!");
        let token = generate_first_run_token(&auth).unwrap();
        let claims = auth.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "admin");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    fn test_claims_display() {
        let claims = Claims {
            sub: "alice".to_string(),
            iat: 1000,
            exp: 2000,
            role: "developer".to_string(),
        };
        assert_eq!(format!("{}", claims), "user=alice, role=developer");
    }
}
