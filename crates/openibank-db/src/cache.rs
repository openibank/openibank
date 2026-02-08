//! Redis cache manager for sessions, rate limiting, and caching

use deadpool_redis::{Pool as RedisPool, redis::AsyncCommands};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

use crate::{DbResult, DbError};

/// Cache key prefixes for organization
pub mod keys {
    pub const SESSION: &str = "session:";
    pub const RATE_LIMIT: &str = "rate:";
    pub const MARKET_DATA: &str = "market:";
    pub const ORDERBOOK: &str = "orderbook:";
    pub const TICKER: &str = "ticker:";
    pub const USER_CACHE: &str = "user:";
    pub const NONCE: &str = "nonce:";
    pub const LOCK: &str = "lock:";
}

/// Default TTLs
pub mod ttl {
    use std::time::Duration;

    pub const SESSION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
    pub const RATE_LIMIT: Duration = Duration::from_secs(60); // 1 minute
    pub const MARKET_DATA: Duration = Duration::from_secs(5); // 5 seconds
    pub const TICKER: Duration = Duration::from_secs(1); // 1 second
    pub const USER_CACHE: Duration = Duration::from_secs(300); // 5 minutes
    pub const NONCE: Duration = Duration::from_secs(300); // 5 minutes
    pub const LOCK: Duration = Duration::from_secs(30); // 30 seconds
}

pub struct CacheManager {
    pool: RedisPool,
}

impl CacheManager {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Basic Operations
    // =========================================================================

    /// Set a value with expiration
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> DbResult<()> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let json = serde_json::to_string(value)
            .map_err(|e| DbError::Serialization(e.to_string()))?;

        conn.set_ex::<_, _, ()>(key, json, ttl.as_secs())
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(())
    }

    /// Get a value
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> DbResult<Option<T>> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let result: Option<String> = conn.get(key)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        match result {
            Some(json) => {
                let value = serde_json::from_str(&json)
                    .map_err(|e| DbError::Serialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Delete a key
    pub async fn delete(&self, key: &str) -> DbResult<bool> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let deleted: i32 = conn.del(key)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(deleted > 0)
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> DbResult<bool> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let exists: bool = conn.exists(key)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(exists)
    }

    /// Set expiration on existing key
    pub async fn expire(&self, key: &str, ttl: Duration) -> DbResult<bool> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let result: bool = conn.expire(key, ttl.as_secs() as i64)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(result)
    }

    // =========================================================================
    // Session Management
    // =========================================================================

    /// Store session data
    pub async fn set_session<T: Serialize>(&self, token: &str, data: &T) -> DbResult<()> {
        let key = format!("{}{}", keys::SESSION, token);
        self.set(&key, data, ttl::SESSION).await
    }

    /// Get session data
    pub async fn get_session<T: DeserializeOwned>(&self, token: &str) -> DbResult<Option<T>> {
        let key = format!("{}{}", keys::SESSION, token);
        self.get(&key).await
    }

    /// Delete session
    pub async fn delete_session(&self, token: &str) -> DbResult<bool> {
        let key = format!("{}{}", keys::SESSION, token);
        self.delete(&key).await
    }

    /// Extend session TTL
    pub async fn extend_session(&self, token: &str) -> DbResult<bool> {
        let key = format!("{}{}", keys::SESSION, token);
        self.expire(&key, ttl::SESSION).await
    }

    // =========================================================================
    // Rate Limiting
    // =========================================================================

    /// Increment rate limit counter, returns current count
    pub async fn rate_limit_incr(&self, identifier: &str, window_secs: u64) -> DbResult<i64> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let key = format!("{}{}", keys::RATE_LIMIT, identifier);

        // Use INCR and set expiry if new key
        let count: i64 = conn.incr(&key, 1)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        if count == 1 {
            // First request in window, set expiry
            let _: () = conn.expire(&key, window_secs as i64)
                .await
                .map_err(|e| DbError::Redis(e.to_string()))?;
        }

        Ok(count)
    }

    /// Check if rate limited (returns remaining requests, -1 if over limit)
    pub async fn check_rate_limit(&self, identifier: &str, limit: i64) -> DbResult<i64> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let key = format!("{}{}", keys::RATE_LIMIT, identifier);

        let count: Option<i64> = conn.get(&key)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let current = count.unwrap_or(0);
        let remaining = limit - current;

        Ok(remaining)
    }

    // =========================================================================
    // Market Data Caching
    // =========================================================================

    /// Cache ticker data
    pub async fn set_ticker<T: Serialize>(&self, market_id: &str, ticker: &T) -> DbResult<()> {
        let key = format!("{}{}", keys::TICKER, market_id);
        self.set(&key, ticker, ttl::TICKER).await
    }

    /// Get cached ticker
    pub async fn get_ticker<T: DeserializeOwned>(&self, market_id: &str) -> DbResult<Option<T>> {
        let key = format!("{}{}", keys::TICKER, market_id);
        self.get(&key).await
    }

    /// Cache orderbook snapshot
    pub async fn set_orderbook<T: Serialize>(&self, market_id: &str, orderbook: &T) -> DbResult<()> {
        let key = format!("{}{}", keys::ORDERBOOK, market_id);
        self.set(&key, orderbook, ttl::MARKET_DATA).await
    }

    /// Get cached orderbook
    pub async fn get_orderbook<T: DeserializeOwned>(&self, market_id: &str) -> DbResult<Option<T>> {
        let key = format!("{}{}", keys::ORDERBOOK, market_id);
        self.get(&key).await
    }

    // =========================================================================
    // Distributed Locking
    // =========================================================================

    /// Acquire a distributed lock
    pub async fn acquire_lock(&self, resource: &str, ttl: Duration) -> DbResult<bool> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let key = format!("{}{}", keys::LOCK, resource);

        // Use SET NX (set if not exists) with expiry
        let result: Option<String> = deadpool_redis::redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(ttl.as_secs())
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(result.is_some())
    }

    /// Release a distributed lock
    pub async fn release_lock(&self, resource: &str) -> DbResult<bool> {
        let key = format!("{}{}", keys::LOCK, resource);
        self.delete(&key).await
    }

    // =========================================================================
    // Nonce Management (for API replay protection)
    // =========================================================================

    /// Check and store nonce (returns true if nonce is new/valid)
    pub async fn check_nonce(&self, api_key_id: &str, nonce: &str) -> DbResult<bool> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let key = format!("{}{}:{}", keys::NONCE, api_key_id, nonce);

        // Use SET NX to atomically check and set
        let result: Option<String> = deadpool_redis::redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(ttl::NONCE.as_secs())
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(result.is_some())
    }

    // =========================================================================
    // Pub/Sub for Real-time Updates
    // =========================================================================

    /// Publish a message to a channel
    pub async fn publish(&self, channel: &str, message: &str) -> DbResult<i64> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let subscribers: i64 = conn.publish(channel, message)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(subscribers)
    }

    // =========================================================================
    // Bulk Operations
    // =========================================================================

    /// Delete multiple keys by pattern (use carefully!)
    pub async fn delete_pattern(&self, pattern: &str) -> DbResult<u64> {
        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        // Use SCAN to find keys matching pattern
        let keys: Vec<String> = deadpool_redis::redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: i64 = conn.del(&keys)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        Ok(deleted as u64)
    }

    /// Get multiple values
    pub async fn mget<T: DeserializeOwned>(&self, keys: &[String]) -> DbResult<Vec<Option<T>>> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.pool.get().await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let results: Vec<Option<String>> = conn.mget(keys)
            .await
            .map_err(|e| DbError::Redis(e.to_string()))?;

        let values = results.into_iter()
            .map(|opt| {
                opt.and_then(|json| serde_json::from_str(&json).ok())
            })
            .collect();

        Ok(values)
    }
}
