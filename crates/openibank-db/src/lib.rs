//! OpeniBank Database Layer
//!
//! Production-grade persistence for the OpeniBank platform using PostgreSQL,
//! TimescaleDB for time-series data, and Redis for caching.
//!
//! # Architecture
//!
//! - **PostgreSQL**: Primary data store for users, wallets, orders, trades
//! - **TimescaleDB**: OHLCV candle hypertables for market data
//! - **Redis**: Session caching, rate limiting, real-time pub/sub
//!
//! # Repository Pattern
//!
//! Each domain has its own repository with CRUD and domain-specific queries.
//! All repositories use SQLx compile-time checked queries.

pub mod config;
pub mod error;
pub mod repos;
pub mod models;
pub mod cache;

use deadpool_redis::redis::AsyncCommands;
use sqlx::postgres::{PgPool, PgPoolOptions};
use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use tracing::info;

pub use config::DatabaseConfig;
pub use error::{DbError, DbResult};
pub use repos::*;
pub use models::*;

/// Database connection pool and caches
pub struct Database {
    /// PostgreSQL connection pool
    pub pg: PgPool,
    /// Redis connection pool
    pub redis: RedisPool,
}

impl Database {
    /// Connect to PostgreSQL and Redis
    pub async fn connect(config: &DatabaseConfig) -> DbResult<Self> {
        info!("Connecting to PostgreSQL: {}", config.postgres_url_masked());
        
        let pg = PgPoolOptions::new()
            .max_connections(config.pg_max_connections)
            .min_connections(config.pg_min_connections)
            .acquire_timeout(std::time::Duration::from_secs(config.pg_acquire_timeout_secs))
            .connect(&config.postgres_url)
            .await
            .map_err(|e| DbError::Connection(format!("PostgreSQL: {}", e)))?;

        info!("Connected to PostgreSQL");

        info!("Connecting to Redis: {}", config.redis_url_masked());
        
        let redis_cfg = RedisConfig::from_url(&config.redis_url);
        let redis = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| DbError::Connection(format!("Redis: {}", e)))?;

        // Test Redis connection
        let mut conn = redis.get().await
            .map_err(|e| DbError::Connection(format!("Redis pool: {}", e)))?;
        let _: String = deadpool_redis::redis::cmd("PING")
            .query_async(&mut *conn)
            .await
            .map_err(|e| DbError::Connection(format!("Redis ping: {}", e)))?;

        info!("Connected to Redis");

        Ok(Self { pg, redis })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> DbResult<()> {
        info!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&self.pg)
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;
        info!("Migrations complete");
        Ok(())
    }

    /// Health check for both databases
    pub async fn health_check(&self) -> DbResult<HealthStatus> {
        // Check PostgreSQL
        let pg_ok = sqlx::query("SELECT 1")
            .fetch_one(&self.pg)
            .await
            .is_ok();

        // Check Redis
        let redis_ok = async {
            let mut conn = self.redis.get().await.ok()?;
            let result: Result<String, _> = deadpool_redis::redis::cmd("PING")
                .query_async(&mut *conn)
                .await;
            result.ok()
        }
        .await
        .is_some();

        Ok(HealthStatus {
            postgres: pg_ok,
            redis: redis_ok,
            healthy: pg_ok && redis_ok,
        })
    }

    /// Create repository instances
    pub fn user_repo(&self) -> UserRepo {
        UserRepo::new(self.pg.clone())
    }

    pub fn wallet_repo(&self) -> WalletRepo {
        WalletRepo::new(self.pg.clone())
    }

    pub fn order_repo(&self) -> OrderRepo {
        OrderRepo::new(self.pg.clone())
    }

    pub fn trade_repo(&self) -> TradeRepo {
        TradeRepo::new(self.pg.clone())
    }

    pub fn market_repo(&self) -> MarketRepo {
        MarketRepo::new(self.pg.clone())
    }

    pub fn candle_repo(&self) -> CandleRepo {
        CandleRepo::new(self.pg.clone())
    }

    pub fn deposit_repo(&self) -> DepositRepo {
        DepositRepo::new(self.pg.clone())
    }

    pub fn withdrawal_repo(&self) -> WithdrawalRepo {
        WithdrawalRepo::new(self.pg.clone())
    }

    pub fn receipt_repo(&self) -> ReceiptRepo {
        ReceiptRepo::new(self.pg.clone())
    }

    pub fn audit_repo(&self) -> AuditRepo {
        AuditRepo::new(self.pg.clone())
    }

    pub fn arena_repo(&self) -> ArenaRepo {
        ArenaRepo::new(self.pg.clone())
    }

    pub fn cache(&self) -> cache::CacheManager {
        cache::CacheManager::new(self.redis.clone())
    }
}

/// Health status of database connections
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub postgres: bool,
    pub redis: bool,
    pub healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_masking() {
        let config = DatabaseConfig {
            postgres_url: "postgresql://user:secret@localhost/db".to_string(),
            redis_url: "redis://:password@localhost:6379".to_string(),
            ..Default::default()
        };

        assert!(!config.postgres_url_masked().contains("secret"));
        assert!(!config.redis_url_masked().contains("password"));
    }
}
