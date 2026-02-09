//! JWT Token Service
//!
//! Production-grade JWT implementation with:
//! - Access tokens (short-lived) for API authentication
//! - Refresh tokens (long-lived) with rotation
//! - Token revocation support
//! - Secure token generation and validation

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::JwtConfig;
use crate::error::{AuthError, AuthResult};
use crate::types::{TokenClaims, TokenPair, TokenType, UserRole};

/// JWT service for token management
#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    /// Set of revoked token IDs (jti)
    /// In production, this should be backed by Redis for distributed systems
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
    /// Set of revoked refresh tokens (for rotation)
    revoked_refresh_tokens: Arc<RwLock<HashSet<String>>>,
}

impl JwtService {
    /// Create a new JWT service
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
            revoked_refresh_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Generate a new token pair (access + refresh)
    pub fn generate_token_pair(
        &self,
        user_id: Uuid,
        email: &str,
        role: UserRole,
        session_id: Option<Uuid>,
        two_factor_verified: bool,
    ) -> AuthResult<TokenPair> {
        let now = Utc::now();
        let access_exp = now + Duration::from_std(self.config.access_token_lifetime)
            .map_err(|e| AuthError::Internal(e.to_string()))?;
        let refresh_exp = now + Duration::from_std(self.config.refresh_token_lifetime)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        // Generate access token
        let access_jti = Uuid::new_v4().to_string();
        let access_claims = TokenClaims {
            sub: user_id.to_string(),
            token_type: TokenType::Access,
            email: email.to_string(),
            role,
            iat: now.timestamp(),
            exp: access_exp.timestamp(),
            nbf: now.timestamp(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            jti: access_jti,
            sid: session_id.map(|s| s.to_string()),
            two_factor_verified,
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &self.encoding_key,
        ).map_err(|e| AuthError::Internal(format!("Failed to encode access token: {}", e)))?;

        // Generate refresh token
        let refresh_jti = Uuid::new_v4().to_string();
        let refresh_claims = TokenClaims {
            sub: user_id.to_string(),
            token_type: TokenType::Refresh,
            email: email.to_string(),
            role,
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
            nbf: now.timestamp(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            jti: refresh_jti,
            sid: session_id.map(|s| s.to_string()),
            two_factor_verified,
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &self.encoding_key,
        ).map_err(|e| AuthError::Internal(format!("Failed to encode refresh token: {}", e)))?;

        Ok(TokenPair::new(
            access_token,
            refresh_token,
            access_exp.timestamp(),
            refresh_exp.timestamp(),
        ))
    }

    /// Validate an access token and return claims
    pub async fn validate_access_token(&self, token: &str) -> AuthResult<TokenClaims> {
        let claims = self.decode_token(token)?;

        // Check token type
        if claims.token_type != TokenType::Access {
            return Err(AuthError::InvalidTokenType);
        }

        // Check if token is revoked
        if self.is_token_revoked(&claims.jti).await {
            return Err(AuthError::TokenRevoked);
        }

        Ok(claims)
    }

    /// Validate a refresh token and return claims
    pub async fn validate_refresh_token(&self, token: &str) -> AuthResult<TokenClaims> {
        let claims = self.decode_token(token)?;

        // Check token type
        if claims.token_type != TokenType::Refresh {
            return Err(AuthError::InvalidTokenType);
        }

        // Check if token is revoked
        if self.is_refresh_token_revoked(&claims.jti).await {
            return Err(AuthError::TokenRevoked);
        }

        Ok(claims)
    }

    /// Refresh tokens using a refresh token
    /// Implements refresh token rotation for security
    pub async fn refresh_tokens(&self, refresh_token: &str) -> AuthResult<TokenPair> {
        let claims = self.validate_refresh_token(refresh_token).await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AuthError::InvalidToken)?;

        // Parse session ID if present
        let session_id = claims.sid.as_ref()
            .and_then(|s| Uuid::parse_str(s).ok());

        // If rotation is enabled, revoke the old refresh token
        if self.config.rotate_refresh_tokens {
            self.revoke_refresh_token(&claims.jti).await;
        }

        // Generate new token pair
        self.generate_token_pair(
            user_id,
            &claims.email,
            claims.role,
            session_id,
            claims.two_factor_verified,
        )
    }

    /// Revoke an access token
    pub async fn revoke_token(&self, jti: &str) {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(jti.to_string());
    }

    /// Revoke a refresh token
    pub async fn revoke_refresh_token(&self, jti: &str) {
        let mut revoked = self.revoked_refresh_tokens.write().await;
        revoked.insert(jti.to_string());
    }

    /// Revoke all tokens for a user session
    pub async fn revoke_session_tokens(&self, session_id: Uuid) {
        // In production, this would query Redis/DB to find all tokens for the session
        // For now, we'll rely on the session service to handle this
        tracing::info!(session_id = %session_id, "Revoking all tokens for session");
    }

    /// Check if a token is revoked
    pub async fn is_token_revoked(&self, jti: &str) -> bool {
        let revoked = self.revoked_tokens.read().await;
        revoked.contains(jti)
    }

    /// Check if a refresh token is revoked
    pub async fn is_refresh_token_revoked(&self, jti: &str) -> bool {
        let revoked = self.revoked_refresh_tokens.read().await;
        revoked.contains(jti)
    }

    /// Decode and validate a token (shared logic)
    fn decode_token(&self, token: &str) -> AuthResult<TokenClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        let token_data = decode::<TokenClaims>(
            token,
            &self.decoding_key,
            &validation,
        )?;

        Ok(token_data.claims)
    }

    /// Extract user ID from token without full validation
    /// Used for logging/metrics when token might be invalid
    pub fn extract_user_id(&self, token: &str) -> Option<Uuid> {
        let mut validation = Validation::default();
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        validation.set_required_spec_claims::<&str>(&[]);
        // Don't validate issuer for extraction
        validation.iss = None;

        decode::<TokenClaims>(token, &self.decoding_key, &validation)
            .ok()
            .and_then(|data| Uuid::parse_str(&data.claims.sub).ok())
    }

    /// Get remaining time until token expiry
    pub fn get_token_ttl(&self, token: &str) -> Option<std::time::Duration> {
        let mut validation = Validation::default();
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.set_required_spec_claims::<&str>(&[]);

        decode::<TokenClaims>(token, &self.decoding_key, &validation)
            .ok()
            .and_then(|data| {
                let exp = chrono::DateTime::from_timestamp(data.claims.exp, 0)?;
                let now = Utc::now();
                if exp > now {
                    Some((exp - now).to_std().ok()?)
                } else {
                    None
                }
            })
    }

    /// Clean up expired revoked tokens (should be called periodically)
    pub async fn cleanup_revoked_tokens(&self) {
        // In production, this would be handled by Redis TTL
        // For in-memory implementation, we'd need to track expiry times
        tracing::debug!("Cleaning up expired revoked tokens");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key-for-jwt-tokens-min-32-bytes!".to_string(),
            access_token_lifetime: std::time::Duration::from_secs(900), // 15 min
            refresh_token_lifetime: std::time::Duration::from_secs(604800), // 7 days
            issuer: "test-issuer".to_string(),
            audience: "test-audience".to_string(),
            algorithm: "HS256".to_string(),
            rotate_refresh_tokens: true,
            refresh_grace_period: std::time::Duration::from_secs(60),
        }
    }

    #[test]
    fn test_generate_token_pair() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let result = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            Some(Uuid::new_v4()),
            false,
        );

        assert!(result.is_ok());
        let pair = result.unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.token_type, "Bearer");
    }

    #[tokio::test]
    async fn test_validate_access_token() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let pair = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            None,
            true,
        ).unwrap();

        let claims = service.validate_access_token(&pair.access_token).await;
        assert!(claims.is_ok());

        let claims = claims.unwrap();
        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, UserRole::User);
        assert!(claims.two_factor_verified);
    }

    #[tokio::test]
    async fn test_validate_refresh_token() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let pair = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            None,
            false,
        ).unwrap();

        let claims = service.validate_refresh_token(&pair.refresh_token).await;
        assert!(claims.is_ok());
        assert_eq!(claims.unwrap().token_type, TokenType::Refresh);
    }

    #[tokio::test]
    async fn test_access_token_fails_as_refresh() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let pair = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            None,
            false,
        ).unwrap();

        // Access token should fail validation as refresh token
        let result = service.validate_refresh_token(&pair.access_token).await;
        assert!(matches!(result, Err(AuthError::InvalidTokenType)));
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let pair = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            None,
            false,
        ).unwrap();

        // Token should be valid initially
        let claims = service.validate_access_token(&pair.access_token).await.unwrap();

        // Revoke the token
        service.revoke_token(&claims.jti).await;

        // Token should now be rejected
        let result = service.validate_access_token(&pair.access_token).await;
        assert!(matches!(result, Err(AuthError::TokenRevoked)));
    }

    #[tokio::test]
    async fn test_refresh_token_rotation() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let pair = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            None,
            false,
        ).unwrap();

        // Refresh tokens
        let new_pair = service.refresh_tokens(&pair.refresh_token).await.unwrap();
        assert!(!new_pair.access_token.is_empty());
        assert!(!new_pair.refresh_token.is_empty());

        // Old refresh token should be revoked (rotation enabled)
        let result = service.validate_refresh_token(&pair.refresh_token).await;
        assert!(matches!(result, Err(AuthError::TokenRevoked)));
    }

    #[test]
    fn test_extract_user_id() {
        let service = JwtService::new(test_config());
        let user_id = Uuid::new_v4();

        let pair = service.generate_token_pair(
            user_id,
            "test@example.com",
            UserRole::User,
            None,
            false,
        ).unwrap();

        let extracted = service.extract_user_id(&pair.access_token);
        assert_eq!(extracted, Some(user_id));
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new(test_config());
        let result = service.decode_token("invalid-token");
        assert!(result.is_err());
    }
}
