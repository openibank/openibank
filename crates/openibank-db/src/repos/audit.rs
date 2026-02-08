//! Audit log repository

use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbResult, DbAuditLog};

pub struct AuditRepo {
    pool: PgPool,
}

impl AuditRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn log(
        &self,
        user_id: Option<Uuid>,
        action: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        details: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> DbResult<DbAuditLog> {
        let log = sqlx::query_as::<_, DbAuditLog>(
            r#"
            INSERT INTO audit_log (user_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(action)
        .bind(resource_type)
        .bind(resource_id)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .fetch_one(&self.pool)
        .await?;
        Ok(log)
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<DbAuditLog>> {
        let log = sqlx::query_as::<_, DbAuditLog>("SELECT * FROM audit_log WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(log)
    }

    pub async fn list_by_user(
        &self,
        user_id: Uuid,
        action: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<DbAuditLog>> {
        let logs = if let Some(act) = action {
            sqlx::query_as::<_, DbAuditLog>(
                "SELECT * FROM audit_log WHERE user_id = $1 AND action = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
            )
            .bind(user_id)
            .bind(act)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, DbAuditLog>(
                "SELECT * FROM audit_log WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(logs)
    }

    pub async fn list_by_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: i64,
    ) -> DbResult<Vec<DbAuditLog>> {
        let logs = sqlx::query_as::<_, DbAuditLog>(
            "SELECT * FROM audit_log WHERE resource_type = $1 AND resource_id = $2 ORDER BY created_at DESC LIMIT $3"
        )
        .bind(resource_type)
        .bind(resource_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }

    pub async fn list_recent(&self, limit: i64) -> DbResult<Vec<DbAuditLog>> {
        let logs = sqlx::query_as::<_, DbAuditLog>(
            "SELECT * FROM audit_log ORDER BY created_at DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }

    pub async fn list_security_events(&self, limit: i64) -> DbResult<Vec<DbAuditLog>> {
        let logs = sqlx::query_as::<_, DbAuditLog>(
            r#"
            SELECT * FROM audit_log
            WHERE action IN ('login', 'logout', 'login_failed', 'password_change', 'api_key_created',
                           'api_key_revoked', '2fa_enabled', '2fa_disabled', 'withdrawal_requested')
            ORDER BY created_at DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }

    pub async fn search(
        &self,
        action: Option<&str>,
        resource_type: Option<&str>,
        ip_address: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<DbAuditLog>> {
        let logs = sqlx::query_as::<_, DbAuditLog>(
            r#"
            SELECT * FROM audit_log
            WHERE ($1::text IS NULL OR action = $1)
              AND ($2::text IS NULL OR resource_type = $2)
              AND ($3::text IS NULL OR ip_address = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#
        )
        .bind(action)
        .bind(resource_type)
        .bind(ip_address)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }
}
