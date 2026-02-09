//! Application state shared across handlers
//!
//! Contains database connections, authentication services, and caches.

use std::sync::Arc;
use openibank_auth::AuthService;
use openibank_db::Database;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connections
    pub db: Arc<Database>,
    /// Authentication service
    pub auth: Arc<AuthService>,
}

impl AppState {
    /// Create a new application state
    pub fn new(db: Arc<Database>, auth: Arc<AuthService>) -> Self {
        Self { db, auth }
    }

    /// Create state for testing (with mock services)
    #[cfg(test)]
    pub fn test() -> Self {
        use openibank_auth::AuthConfig;

        let db = Arc::new(Database::new_mock());
        let mut auth_config = AuthConfig::default();
        auth_config.jwt.secret = "test-secret-key-at-least-32-bytes-long!!".to_string();
        let auth = Arc::new(AuthService::new(db.clone(), auth_config));

        Self { db, auth }
    }
}
