//! Database configuration

use serde::{Deserialize, Serialize};

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub postgres_url: String,
    /// Redis connection URL
    pub redis_url: String,
    /// Maximum PostgreSQL connections
    pub pg_max_connections: u32,
    /// Minimum PostgreSQL connections
    pub pg_min_connections: u32,
    /// Connection acquire timeout in seconds
    pub pg_acquire_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://localhost/openibank".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            pg_max_connections: 100,
            pg_min_connections: 5,
            pg_acquire_timeout_secs: 30,
        }
    }
}

impl DatabaseConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        Self {
            postgres_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            redis_url: std::env::var("REDIS_URL")
                .expect("REDIS_URL must be set"),
            pg_max_connections: std::env::var("PG_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            pg_min_connections: std::env::var("PG_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            pg_acquire_timeout_secs: std::env::var("PG_ACQUIRE_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        }
    }

    /// Mask sensitive parts of the PostgreSQL URL for logging
    pub fn postgres_url_masked(&self) -> String {
        mask_url(&self.postgres_url)
    }

    /// Mask sensitive parts of the Redis URL for logging
    pub fn redis_url_masked(&self) -> String {
        mask_url(&self.redis_url)
    }
}

fn mask_url(url: &str) -> String {
    // Simple masking: replace password with ***
    if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let after_at = &url[at_pos..];
            
            // Find if there's a user:pass pattern
            let user_pass = &url[scheme_end + 3..at_pos];
            if let Some(colon_pos) = user_pass.find(':') {
                let user = &user_pass[..colon_pos];
                return format!("{}{}:***{}", scheme, user, after_at);
            }
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_postgres_url() {
        let url = "postgresql://user:secret123@localhost:5432/db";
        let masked = mask_url(url);
        assert_eq!(masked, "postgresql://user:***@localhost:5432/db");
        assert!(!masked.contains("secret123"));
    }

    #[test]
    fn test_mask_redis_url() {
        let url = "redis://:mypassword@localhost:6379";
        let masked = mask_url(url);
        assert!(!masked.contains("mypassword"));
    }

    #[test]
    fn test_no_password() {
        let url = "postgresql://localhost/db";
        let masked = mask_url(url);
        assert_eq!(masked, url);
    }
}
