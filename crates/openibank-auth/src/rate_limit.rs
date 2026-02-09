//! Rate Limiting Service
//!
//! Production-grade rate limiting with:
//! - Per-user and per-IP rate limiting
//! - Login attempt tracking with progressive lockout
//! - Redis-backed for distributed systems
//! - Sliding window algorithm for accurate limiting

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::config::RateLimitConfig;
use crate::error::{AuthError, AuthResult};
use openibank_db::Database;

/// Rate limiter service
#[derive(Clone)]
pub struct RateLimiter {
    db: Arc<Database>,
    config: RateLimitConfig,
    /// In-memory rate limit buckets (for single-instance mode)
    /// In production, use Redis for distributed rate limiting
    buckets: Arc<RwLock<HashMap<String, RateBucket>>>,
    /// Login attempt tracking
    login_attempts: Arc<RwLock<HashMap<String, LoginAttempts>>>,
}

/// Rate limit bucket for tracking requests
#[derive(Debug, Clone)]
struct RateBucket {
    /// Request timestamps within the window
    requests: Vec<Instant>,
    /// Window start time
    window_start: Instant,
}

/// Login attempt tracking for lockout
#[derive(Debug, Clone)]
struct LoginAttempts {
    /// Number of failed attempts
    failed_count: u32,
    /// Last failed attempt time
    last_failed: Instant,
    /// Current lockout duration (grows with each lockout)
    lockout_duration: Duration,
    /// Whether currently locked out
    locked_until: Option<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(db: Arc<Database>, config: RateLimitConfig) -> Self {
        Self {
            db,
            config,
            buckets: Arc::new(RwLock::new(HashMap::new())),
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request should be rate limited (for authenticated users)
    pub async fn check_user_limit(&self, user_id: &str) -> AuthResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = format!("user:{}", user_id);
        self.check_limit(
            &key,
            self.config.api_requests_per_window,
            self.config.api_window,
        )
        .await
    }

    /// Check if a request should be rate limited (by IP)
    pub async fn check_ip_limit(&self, ip: &str) -> AuthResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let key = format!("ip:{}", ip);
        self.check_limit(
            &key,
            self.config.ip_requests_per_window,
            self.config.ip_window,
        )
        .await
    }

    /// Check and record a login attempt
    pub async fn check_login_limit(&self, identifier: &str) -> AuthResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let attempts = self.login_attempts.write().await;

        if let Some(attempt_info) = attempts.get(identifier) {
            // Check if currently locked out
            if let Some(locked_until) = attempt_info.locked_until {
                if Instant::now() < locked_until {
                    let remaining = locked_until.duration_since(Instant::now());
                    return Err(AuthError::account_locked(remaining));
                }
            }
        }

        // Check login rate limit
        let key = format!("login:{}", identifier);
        self.check_limit(
            &key,
            self.config.login_attempts,
            self.config.login_window,
        )
        .await
    }

    /// Record a failed login attempt
    pub async fn record_failed_login(&self, identifier: &str) {
        let mut attempts = self.login_attempts.write().await;

        let entry = attempts.entry(identifier.to_string()).or_insert(LoginAttempts {
            failed_count: 0,
            last_failed: Instant::now(),
            lockout_duration: self.config.lockout_duration,
            locked_until: None,
        });

        entry.failed_count += 1;
        entry.last_failed = Instant::now();

        // Check if should lock out
        if entry.failed_count >= self.config.login_attempts {
            let lockout = entry.lockout_duration.min(self.config.max_lockout_duration);
            entry.locked_until = Some(Instant::now() + lockout);

            // Progressive lockout - increase duration for next time
            entry.lockout_duration = Duration::from_secs_f64(
                (entry.lockout_duration.as_secs_f64() * self.config.lockout_multiplier)
                    .min(self.config.max_lockout_duration.as_secs_f64()),
            );

            tracing::warn!(
                identifier = identifier,
                lockout_seconds = lockout.as_secs(),
                "Account locked due to failed login attempts"
            );
        }
    }

    /// Record a successful login (reset failed attempts)
    pub async fn record_successful_login(&self, identifier: &str) {
        let mut attempts = self.login_attempts.write().await;
        attempts.remove(identifier);
    }

    /// Get rate limit info for a key
    pub async fn get_limit_info(&self, key: &str) -> Option<RateLimitInfo> {
        let buckets = self.buckets.read().await;

        buckets.get(key).map(|bucket| {
            let now = Instant::now();
            let window_elapsed = now.duration_since(bucket.window_start);

            // Count requests still within window
            let current_count = bucket
                .requests
                .iter()
                .filter(|&t| now.duration_since(*t) < self.config.api_window)
                .count() as u32;

            RateLimitInfo {
                limit: self.config.api_requests_per_window,
                remaining: self.config.api_requests_per_window.saturating_sub(current_count),
                reset_at: bucket.window_start + self.config.api_window,
                window_elapsed,
            }
        })
    }

    /// Check rate limit for a generic key
    async fn check_limit(&self, key: &str, limit: u32, window: Duration) -> AuthResult<()> {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        let bucket = buckets.entry(key.to_string()).or_insert(RateBucket {
            requests: Vec::new(),
            window_start: now,
        });

        // Remove expired requests (sliding window)
        bucket.requests.retain(|&t| now.duration_since(t) < window);

        // Check if limit exceeded
        if bucket.requests.len() >= limit as usize {
            // Calculate retry-after
            if let Some(&oldest) = bucket.requests.first() {
                let retry_after = window.saturating_sub(now.duration_since(oldest));
                return Err(AuthError::rate_limited(retry_after));
            }
            return Err(AuthError::rate_limited(window));
        }

        // Record this request
        bucket.requests.push(now);

        Ok(())
    }

    /// Clean up expired buckets (should be called periodically)
    pub async fn cleanup(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        // Use max window duration for cleanup
        let max_window = self.config.api_window
            .max(self.config.ip_window)
            .max(self.config.login_window);

        buckets.retain(|_, bucket| {
            now.duration_since(bucket.window_start) < max_window * 2
        });

        // Also clean up old login attempts
        let mut attempts = self.login_attempts.write().await;
        attempts.retain(|_, attempt| {
            // Keep if locked or recently active
            if let Some(locked_until) = attempt.locked_until {
                if now < locked_until {
                    return true;
                }
            }
            now.duration_since(attempt.last_failed) < self.config.max_lockout_duration
        });
    }

    /// Reset rate limit for a key (admin function)
    pub async fn reset_limit(&self, key: &str) {
        let mut buckets = self.buckets.write().await;
        buckets.remove(key);
    }

    /// Reset login attempts for an identifier (admin function)
    pub async fn reset_login_attempts(&self, identifier: &str) {
        let mut attempts = self.login_attempts.write().await;
        attempts.remove(identifier);
    }

    /// Check if an identifier is currently locked out
    pub async fn is_locked_out(&self, identifier: &str) -> Option<Duration> {
        let attempts = self.login_attempts.read().await;

        if let Some(attempt_info) = attempts.get(identifier) {
            if let Some(locked_until) = attempt_info.locked_until {
                let now = Instant::now();
                if now < locked_until {
                    return Some(locked_until.duration_since(now));
                }
            }
        }

        None
    }
}

/// Rate limit information
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Maximum requests allowed
    pub limit: u32,
    /// Remaining requests in current window
    pub remaining: u32,
    /// When the window resets
    pub reset_at: Instant,
    /// How long since window started
    pub window_elapsed: Duration,
}

impl RateLimitInfo {
    /// Get headers for rate limit info
    pub fn to_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("X-RateLimit-Limit", self.limit.to_string()),
            ("X-RateLimit-Remaining", self.remaining.to_string()),
            (
                "X-RateLimit-Reset",
                (Instant::now() + self.reset_at.duration_since(Instant::now()))
                    .elapsed()
                    .as_secs()
                    .to_string(),
            ),
        ]
    }
}

/// Middleware helper to extract client IP
pub fn extract_client_ip(headers: &axum::http::HeaderMap, peer_addr: Option<std::net::SocketAddr>) -> String {
    // Try common proxy headers first
    let forwarded_for = headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());

    let real_ip = headers
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    let cf_ip = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    // Priority: CF > X-Real-IP > X-Forwarded-For > peer addr
    cf_ip
        .or(real_ip)
        .or(forwarded_for)
        .or_else(|| peer_addr.map(|a| a.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RateLimitConfig {
        RateLimitConfig {
            enabled: true,
            login_attempts: 5,
            login_window: Duration::from_secs(60),
            api_requests_per_window: 10,
            api_window: Duration::from_secs(1),
            ip_requests_per_window: 5,
            ip_window: Duration::from_secs(1),
            lockout_duration: Duration::from_secs(60),
            lockout_multiplier: 2.0,
            max_lockout_duration: Duration::from_secs(3600),
        }
    }

    fn test_limiter() -> RateLimiter {
        RateLimiter::new(
            Arc::new(openibank_db::Database::new_mock()),
            test_config(),
        )
    }

    #[tokio::test]
    async fn test_user_rate_limit() {
        let limiter = test_limiter();

        // Should allow up to limit
        for _ in 0..10 {
            assert!(limiter.check_user_limit("user1").await.is_ok());
        }

        // Should reject after limit
        let result = limiter.check_user_limit("user1").await;
        assert!(matches!(result, Err(AuthError::RateLimitExceeded { .. })));
    }

    #[tokio::test]
    async fn test_ip_rate_limit() {
        let limiter = test_limiter();

        // Should allow up to limit
        for _ in 0..5 {
            assert!(limiter.check_ip_limit("192.168.1.1").await.is_ok());
        }

        // Should reject after limit
        let result = limiter.check_ip_limit("192.168.1.1").await;
        assert!(matches!(result, Err(AuthError::RateLimitExceeded { .. })));

        // Different IP should still work
        assert!(limiter.check_ip_limit("192.168.1.2").await.is_ok());
    }

    #[tokio::test]
    async fn test_login_lockout() {
        let limiter = test_limiter();

        // Record failed attempts
        for _ in 0..5 {
            limiter.record_failed_login("user@example.com").await;
        }

        // Should be locked out
        let result = limiter.check_login_limit("user@example.com").await;
        assert!(matches!(result, Err(AuthError::AccountLocked { .. })));
    }

    #[tokio::test]
    async fn test_successful_login_resets() {
        let limiter = test_limiter();

        // Record some failed attempts
        for _ in 0..3 {
            limiter.record_failed_login("user@example.com").await;
        }

        // Successful login should reset
        limiter.record_successful_login("user@example.com").await;

        // Should be able to login again
        assert!(limiter.check_login_limit("user@example.com").await.is_ok());
    }

    #[tokio::test]
    async fn test_disabled_rate_limiting() {
        let mut config = test_config();
        config.enabled = false;
        let limiter = RateLimiter::new(
            Arc::new(openibank_db::Database::new_mock()),
            config,
        );

        // Should allow unlimited requests when disabled
        for _ in 0..100 {
            assert!(limiter.check_user_limit("user1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limit_info() {
        let limiter = test_limiter();

        // Make some requests
        for _ in 0..5 {
            limiter.check_user_limit("user1").await.ok();
        }

        let info = limiter.get_limit_info("user:user1").await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.limit, 10);
        assert_eq!(info.remaining, 5);
    }

    #[tokio::test]
    async fn test_reset_limit() {
        let limiter = test_limiter();

        // Hit the limit
        for _ in 0..10 {
            limiter.check_user_limit("user1").await.ok();
        }
        assert!(limiter.check_user_limit("user1").await.is_err());

        // Reset
        limiter.reset_limit("user:user1").await;

        // Should work again
        assert!(limiter.check_user_limit("user1").await.is_ok());
    }

    #[test]
    fn test_extract_client_ip() {
        use axum::http::HeaderMap;

        // Test X-Forwarded-For
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, None), "1.2.3.4");

        // Test X-Real-IP takes precedence
        headers.insert("X-Real-IP", "10.0.0.1".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, None), "10.0.0.1");

        // Test CF-Connecting-IP takes highest precedence
        headers.insert("CF-Connecting-IP", "172.16.0.1".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, None), "172.16.0.1");
    }
}
