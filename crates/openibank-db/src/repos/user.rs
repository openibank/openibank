//! User repository

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{DbResult, DbError, DbUser, DbApiKey, DbSession};

/// User repository for authentication and profile management
pub struct UserRepo {
    pool: PgPool,
}

impl UserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new user
    pub async fn create(
        &self,
        email: &str,
        password_hash: &str,
        referral_code: &str,
        referred_by: Option<Uuid>,
    ) -> DbResult<DbUser> {
        let user = sqlx::query_as::<_, DbUser>(
            r#"
            INSERT INTO users (email, password_hash, referral_code, referred_by)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id, email, email_verified, password_hash, username, phone, phone_verified,
                kyc_tier, status, referral_code, referred_by, anti_phishing_code,
                locale, timezone, created_at, updated_at, deleted_at
            "#
        )
        .bind(email)
        .bind(password_hash)
        .bind(referral_code)
        .bind(referred_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.constraint() == Some("users_email_key") {
                    return DbError::Duplicate(format!("Email {} already exists", email));
                }
            }
            DbError::Query(e)
        })?;

        Ok(user)
    }

    /// Find user by ID
    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbUser>> {
        let user = sqlx::query_as::<_, DbUser>(
            r#"
            SELECT
                id, email, email_verified, password_hash, username, phone, phone_verified,
                kyc_tier, status, referral_code, referred_by, anti_phishing_code,
                locale, timezone, created_at, updated_at, deleted_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> DbResult<Option<DbUser>> {
        let user = sqlx::query_as::<_, DbUser>(
            r#"
            SELECT
                id, email, email_verified, password_hash, username, phone, phone_verified,
                kyc_tier, status, referral_code, referred_by, anti_phishing_code,
                locale, timezone, created_at, updated_at, deleted_at
            FROM users
            WHERE email = $1 AND deleted_at IS NULL
            "#
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find user by referral code
    pub async fn find_by_referral_code(&self, code: &str) -> DbResult<Option<DbUser>> {
        let user = sqlx::query_as::<_, DbUser>(
            r#"
            SELECT
                id, email, email_verified, password_hash, username, phone, phone_verified,
                kyc_tier, status, referral_code, referred_by, anti_phishing_code,
                locale, timezone, created_at, updated_at, deleted_at
            FROM users
            WHERE referral_code = $1 AND deleted_at IS NULL
            "#
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Update user email verification status
    pub async fn verify_email(&self, user_id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update user password
    pub async fn update_password(&self, user_id: Uuid, password_hash: &str) -> DbResult<()> {
        sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
            .bind(user_id)
            .bind(password_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update KYC tier
    pub async fn update_kyc_tier(&self, user_id: Uuid, tier: i16) -> DbResult<()> {
        sqlx::query("UPDATE users SET kyc_tier = $2 WHERE id = $1")
            .bind(user_id)
            .bind(tier)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update user status
    pub async fn update_status(&self, user_id: Uuid, status: &str) -> DbResult<()> {
        sqlx::query("UPDATE users SET status = $2 WHERE id = $1")
            .bind(user_id)
            .bind(status)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Soft delete user
    pub async fn soft_delete(&self, user_id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE users SET deleted_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // API Keys
    // =========================================================================

    /// Create API key
    pub async fn create_api_key(
        &self,
        user_id: Uuid,
        key_hash: &str,
        secret_hash: &str,
        label: Option<&str>,
        permissions: serde_json::Value,
    ) -> DbResult<DbApiKey> {
        let key = sqlx::query_as::<_, DbApiKey>(
            r#"
            INSERT INTO api_keys (user_id, key_hash, secret_hash, label, permissions)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, user_id, key_hash, secret_hash, label, permissions,
                ip_whitelist, expires_at, last_used_at, created_at, revoked_at
            "#
        )
        .bind(user_id)
        .bind(key_hash)
        .bind(secret_hash)
        .bind(label)
        .bind(permissions)
        .fetch_one(&self.pool)
        .await?;

        Ok(key)
    }

    /// Find API key by hash
    pub async fn find_api_key_by_hash(&self, key_hash: &str) -> DbResult<Option<DbApiKey>> {
        let key = sqlx::query_as::<_, DbApiKey>(
            r#"
            SELECT
                id, user_id, key_hash, secret_hash, label, permissions,
                ip_whitelist, expires_at, last_used_at, created_at, revoked_at
            FROM api_keys
            WHERE key_hash = $1 AND revoked_at IS NULL
            "#
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(key)
    }

    /// List user's API keys
    pub async fn list_api_keys(&self, user_id: Uuid) -> DbResult<Vec<DbApiKey>> {
        let keys = sqlx::query_as::<_, DbApiKey>(
            r#"
            SELECT
                id, user_id, key_hash, secret_hash, label, permissions,
                ip_whitelist, expires_at, last_used_at, created_at, revoked_at
            FROM api_keys
            WHERE user_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Revoke API key
    pub async fn revoke_api_key(&self, key_id: Uuid, user_id: Uuid) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE api_keys SET revoked_at = NOW() WHERE id = $1 AND user_id = $2"
        )
        .bind(key_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound("API key not found".to_string()));
        }

        Ok(())
    }

    /// Update last used timestamp
    pub async fn touch_api_key(&self, key_id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // Sessions
    // =========================================================================

    /// Create session
    pub async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        device_name: Option<&str>,
        device_type: Option<&str>,
        expires_at: DateTime<Utc>,
    ) -> DbResult<DbSession> {
        let session = sqlx::query_as::<_, DbSession>(
            r#"
            INSERT INTO user_sessions (user_id, token_hash, ip_address, user_agent, device_name, device_type, expires_at)
            VALUES ($1, $2, $3::inet, $4, $5, $6, $7)
            RETURNING
                id, user_id, token_hash,
                ip_address::text as ip_address,
                user_agent, device_name, device_type, expires_at, created_at
            "#
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(ip_address)
        .bind(user_agent)
        .bind(device_name)
        .bind(device_type)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Find session by token hash
    pub async fn find_session_by_token(&self, token_hash: &str) -> DbResult<Option<DbSession>> {
        let session = sqlx::query_as::<_, DbSession>(
            r#"
            SELECT
                id, user_id, token_hash,
                ip_address::text as ip_address,
                user_agent, device_name, device_type, expires_at, created_at
            FROM user_sessions
            WHERE token_hash = $1 AND expires_at > NOW()
            "#
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// List user's active sessions
    pub async fn list_sessions(&self, user_id: Uuid) -> DbResult<Vec<DbSession>> {
        let sessions = sqlx::query_as::<_, DbSession>(
            r#"
            SELECT
                id, user_id, token_hash,
                ip_address::text as ip_address,
                user_agent, device_name, device_type, expires_at, created_at
            FROM user_sessions
            WHERE user_id = $1 AND expires_at > NOW()
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: Uuid, user_id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM user_sessions WHERE id = $1 AND user_id = $2")
            .bind(session_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete all sessions for user (logout everywhere)
    pub async fn delete_all_sessions(&self, user_id: Uuid) -> DbResult<u64> {
        let result = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> DbResult<u64> {
        let result = sqlx::query("DELETE FROM user_sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
