//! Database error types

use thiserror::Error;

/// Database operation errors
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Query error: {0}")]
    Query(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Duplicate: {0}")]
    Duplicate(String),

    #[error("Constraint violation: {0}")]
    Constraint(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
}

impl From<deadpool_redis::PoolError> for DbError {
    fn from(e: deadpool_redis::PoolError) -> Self {
        DbError::Redis(e.to_string())
    }
}

impl From<redis::RedisError> for DbError {
    fn from(e: redis::RedisError) -> Self {
        DbError::Redis(e.to_string())
    }
}

impl From<serde_json::Error> for DbError {
    fn from(e: serde_json::Error) -> Self {
        DbError::Serialization(e.to_string())
    }
}

/// Result type for database operations
pub type DbResult<T> = Result<T, DbError>;
